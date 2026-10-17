use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use synt_client::{Client, ClientError};
use synt_core::{
    Broker, Core, CoreCaller, CoreScreen, EventFeed, ExtensionMerger, ProjectList, ProjectOpen,
    Projects, SecretList, SecretPut, SecretRemove, Secrets,
};
use synt_extensions::{Extensions, Remote, Runtime};
use synt_journal::{Event, EventKind, Filter, Seq};
use synt_proto::{ActionCall, ActionOutcome, ApprovalDecision, Notice, Notification};
use synt_sync::Syncer;
use synt_testkit::TestError;
use synt_testkit::core::{Approver, Grounds, Harness, register_sync};
use synt_testkit::extensions::everything_component;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use super::EchoServer;

pub const PROJECT: &str = "orders";
const PROJECT_FILE: &str = "id = \"0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e\"\nname = \"Orders\"\n";

const UNUSED_REGISTRY: &str = "http://127.0.0.1:9";
const UNUSED_KEY: &str = "RWQOuk/x7vRoqs2E6Vxffy293iLd8y+C5Yo2oilreJ2V22RL3qFeV7f2";

const MANIFEST: &str = r#"
id = "every"
name = "Everything"
version = "0.2.0"
api = "0.2"
description = "Uses every capability of extension api 0.2."

[capabilities]
call = ["core.ping"]
storage = true
http = ["127.0.0.1:PORT"]
secrets = { "test/token" = ["127.0.0.1:PORT"] }
documents = true
events = ["journal", "approval"]
timers = true
files = "read-write"

[[contributes.sidebar]]
id = "overview"
title = "Everything"
view = "overview"

[[contributes.document]]
type = "note"
title = "Note"
suffix = "note"
panel = "editor"

[[contributes.panel]]
id = "editor"
title = "Note editor"
entry = "index.html"

[[contributes.command]]
title = "Ping the core"
action = "every.ping"

[[contributes.resource.menu]]
title = "Ping from here"
action = "every.ping"
kinds = ["host"]

[[contributes.status]]
id = "beat"
title = "Beat"
view = "beat"

[contributes.settings.schema]
type = "object"
properties = { greeting = { type = "string" }, every = { type = "integer" } }
"#;

pub struct Stage {
    pub harness: Harness,
    pub echo: EchoServer,
    pub approver: Approver,
    pub project: PathBuf,
    syncer: Syncer,
    _feed: [JoinHandle<()>; 2],
}

impl Stage {
    pub async fn start() -> Result<Self, TestError> {
        let grounds = Grounds::new()?;
        let echo = EchoServer::start()?;
        let (core, extensions, projects, syncer) = assemble(&grounds)?;

        extensions.connect(
            Arc::new(CoreCaller::new(&core)),
            Arc::new(CoreScreen::new(&core)),
        );
        let feed = EventFeed::start(&extensions, &grounds.journal(), &projects);

        let harness = Harness::serve(grounds, core)?;
        let approver = harness.approver().await?;
        let project = harness.grounds().path().join(PROJECT);
        fs::create_dir(&project)?;
        fs::write(project.join("synt.toml"), PROJECT_FILE)?;

        let mut stage = Self {
            harness,
            echo,
            approver,
            project,
            syncer,
            _feed: feed,
        };
        stage.open_project().await?;
        stage.install().await?;

        Ok(stage)
    }

    pub fn app(&self) -> &Client {
        self.approver.app()
    }

    pub async fn call(&self, action: &str, params: Value) -> Result<Value, TestError> {
        Ok(self.try_call(action, params).await?.value)
    }

    pub async fn try_call(
        &self,
        action: &str,
        params: Value,
    ) -> Result<ActionOutcome, ClientError> {
        self.app()
            .call(ActionCall::new(action).params(params))
            .await
    }

    pub async fn every(&self, action: &str, params: Value) -> Result<Value, TestError> {
        self.call(&format!("every.{action}"), params).await
    }

    pub async fn try_every(
        &self,
        action: &str,
        params: Value,
    ) -> Result<ActionOutcome, ClientError> {
        self.try_call(&format!("every.{action}"), params).await
    }

    pub async fn watcher(&self) -> Result<(Client, broadcast::Receiver<Notification>), TestError> {
        let app = self.harness.app().await?;
        let notifications = app.notifications();

        Ok((app, notifications))
    }

    pub fn events(&self) -> Result<Vec<Event>, TestError> {
        Ok(self.harness.events()?)
    }

    pub fn journal_text(&self) -> Result<String, TestError> {
        Ok(self
            .events()?
            .iter()
            .map(|event| format!("{} {}", event.action(), event.details().value()))
            .collect::<Vec<_>>()
            .join("\n"))
    }

    pub fn trail(&self, request: Seq) -> Result<Vec<Event>, TestError> {
        Ok(self.harness.matching(&Filter::all().call(request))?)
    }

    pub fn nested(&self, action: &str) -> Result<Vec<Event>, TestError> {
        Ok(self
            .events()?
            .into_iter()
            .filter(|event| event.kind() == EventKind::Requested)
            .filter(|event| event.action().as_str() == action)
            .collect())
    }

    pub async fn sync(&self) {
        for report in self.syncer.sync_now().await {
            assert_eq!(report.error, None, "a sync round failed");
        }
    }

    pub async fn put_secret(&self, name: &str, value: &str) -> Result<(), TestError> {
        self.call("secret.put", json!({ "name": name, "value": value }))
            .await?;

        Ok(())
    }

    async fn open_project(&self) -> Result<(), TestError> {
        self.call("project.open", json!({ "path": self.project }))
            .await?;

        Ok(())
    }

    async fn install(&mut self) -> Result<(), TestError> {
        let build = self.harness.grounds().path().join("every-build");
        fs::create_dir(&build)?;
        fs::write(build.join("extension.wasm"), everything_component())?;
        fs::write(
            build.join("extension.toml"),
            MANIFEST.replace("PORT", &self.echo.port().to_string()),
        )?;

        let app = self.approver.app().clone();
        let call = ActionCall::new("extension.install_local").params(json!({ "path": build }));

        self.approver
            .approve(&app, call, ApprovalDecision::AllowOnce)
            .await?;

        Ok(())
    }
}

pub fn read<T: Notice>(notification: &Notification) -> Option<T> {
    notification.read::<T>().and_then(Result::ok)
}

fn assemble(grounds: &Grounds) -> Result<(Arc<Core>, Extensions, Projects, Syncer), TestError> {
    let projects = Projects::start(grounds.state()?)?;
    let secrets = Secrets::new(grounds.vault(), grounds.state()?);

    let mut registry = grounds.registry()?;
    registry.register(ProjectOpen::new(projects.clone()))?;
    registry.register(ProjectList::new(projects.clone()))?;
    registry.register(SecretPut::new(secrets.clone()))?;
    registry.register(SecretList::new(secrets.clone()))?;
    registry.register(SecretRemove::new(secrets.clone()))?;

    let extensions = Extensions::open(
        &grounds.path().join("data"),
        &grounds.state_path(),
        Runtime::new()?,
        Remote::new(UNUSED_REGISTRY, UNUSED_KEY)?,
    )?
    .with_secrets(Arc::new(secrets))
    .with_projects(Arc::new(projects.clone()));

    let syncer = Syncer::builder(
        &grounds.state_path(),
        grounds.vault(),
        Arc::new(projects.clone()),
    )
    .journal(grounds.journal())
    .merger(Arc::new(ExtensionMerger::new(extensions.clone())))
    .open()?;
    register_sync(&mut registry, &syncer)?;
    extensions.register(&mut registry)?;

    let core = Arc::new(Core::new(registry, grounds.journal(), Broker::new()));

    Ok((core, extensions, projects, syncer))
}
