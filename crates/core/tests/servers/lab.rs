use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Map, Value, json};
use synt_actions::{ActionGroup, Registry};
use synt_client::{Autostart, Client, ClientError};
use synt_core::{
    Broker, Core, DbActions, ProjectOpen, Projects, Resource, RunbookActions, Runbooks, SecretPut,
    Secrets, SessionBlocks, SessionClose, SessionList, SftpActions, SshConnections, SshExec,
    SshKeys, SshSessionOpen, State, Terminals, TunnelClose, TunnelOpen,
};
use synt_db::{Engine, TlsMode};
use synt_journal::{Event, EventKind, Filter, Journal, JournalWriter, MemoryKeyStore, Seq};
use synt_platform::Endpoint;
use synt_proto::{ActionCall, ActionOutcome, ClientKind, Hello, RequestSeq};
use synt_ssh::{Pool, Tunnels};
use synt_test_servers::{DbServers, SshHome, SshServers};
use synt_testkit::MemoryVault;
use synt_testkit::TestError;
use synt_testkit::core::RunningServer;
use tempfile::TempDir;
use tokio::time;

use crate::Person;

pub const PROJECT: &str = "lab";
pub const BASTION: &str = "@lab/bastion";
pub const INNER: &str = "@lab/inner";
pub const VAULTED: &str = "@lab/vaulted";
pub const ECHO: &str = "@lab/echo";
pub const POSTGRES: &str = "@lab/postgres";
pub const POSTGRES_PASSWORD: &str = "lab-postgres";
const POSTGRES_HOST: &str = "db-postgres";

const PROJECT_ID: &str = "5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e";
const DEFAULT_GRACE: Duration = Duration::from_secs(30);
const PAGE: u32 = 10_000;
const WAIT: Duration = Duration::from_secs(20);

pub struct Lab {
    pub servers: SshServers,
    _home: SshHome,
    pub keys: SshKeys,
    pub client: Client,
    journal: JournalWriter,
    running: RunningServer,
    _person: Person,
    _directory: TempDir,
}

impl Lab {
    pub async fn start() -> Result<Option<Self>, TestError> {
        Self::build(SshServers::trusting_home, DEFAULT_GRACE).await
    }

    pub async fn knowing(
        home: impl FnOnce(&SshServers) -> SshHome,
    ) -> Result<Option<Self>, TestError> {
        Self::build(home, DEFAULT_GRACE).await
    }

    pub async fn with_grace(grace: Duration) -> Result<Option<Self>, TestError> {
        Self::build(SshServers::trusting_home, grace).await
    }

    pub async fn exec(&self, host: &str, command: &str) -> Result<ActionOutcome, ClientError> {
        self.call(host, "ssh.exec", json!({ "command": command }))
            .await
    }

    pub async fn call(
        &self,
        host: &str,
        action: &str,
        params: Value,
    ) -> Result<ActionOutcome, ClientError> {
        let call = ActionCall::new(action).target(host).params(params);

        self.client.call(call).await
    }

    pub async fn client_port(&self, host: &str) -> Result<String, TestError> {
        let outcome = self.exec(host, "printenv SSH_CLIENT").await?;
        let line = outcome.value["stdout"]
            .as_str()
            .ok_or("ssh.exec gave no stdout")?;
        let port = line
            .split_whitespace()
            .nth(1)
            .ok_or("SSH_CLIENT names no port")?;

        Ok(port.to_owned())
    }

    pub fn trail(&self, request: RequestSeq) -> Result<Vec<Event>, TestError> {
        self.events(&Filter::all().call(Seq::new(request.get())))
    }

    pub fn kinds(&self, request: RequestSeq) -> Result<Vec<EventKind>, TestError> {
        Ok(self.trail(request)?.iter().map(Event::kind).collect())
    }

    pub async fn finished(&self, request: RequestSeq) -> Result<Value, TestError> {
        self.settled(request, EventKind::Finished).await
    }

    pub fn last_failure(&self, action: &str) -> Result<Value, TestError> {
        self.last_of(action, EventKind::Failed)
    }

    pub fn last_approval(&self, action: &str) -> Result<Value, TestError> {
        self.last_of(action, EventKind::Approved)
    }

    pub async fn stop(self) -> Result<(), TestError> {
        let Self {
            client,
            running,
            _person: person,
            ..
        } = self;

        drop(person);
        drop(client);

        running.stop().await?;

        Ok(())
    }

    async fn settled(&self, request: RequestSeq, kind: EventKind) -> Result<Value, TestError> {
        time::timeout(WAIT, async {
            loop {
                let found = self
                    .trail(request)?
                    .into_iter()
                    .find(|event| event.kind() == kind);

                if let Some(event) = found {
                    return Ok(event.details().value().clone());
                }

                time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .map_err(|_| format!("the journal never recorded {kind:?} for {request:?}"))?
    }

    fn last_of(&self, action: &str, kind: EventKind) -> Result<Value, TestError> {
        let events = self.events(&Filter::all())?;

        let last = events
            .iter()
            .rev()
            .find(|event| event.kind() == kind && event.action().as_str() == action)
            .ok_or_else(|| format!("the journal holds no {kind:?} event of {action}"))?;

        Ok(last.details().value().clone())
    }

    fn events(&self, filter: &Filter) -> Result<Vec<Event>, TestError> {
        let reader = self.journal.reader()?;

        Ok(reader.page(filter, None, PAGE)?)
    }

    async fn build(
        home: impl FnOnce(&SshServers) -> SshHome,
        grace: Duration,
    ) -> Result<Option<Self>, TestError> {
        let Some(servers) = SshServers::running() else {
            return Ok(None);
        };
        let home = home(&servers);
        let directory = tempfile::tempdir()?;

        let journal = journal(directory.path())?;
        let state = State::open(&directory.path().join("state.sqlite"))?;
        let projects = Projects::start(state.clone())?;
        let vault = Arc::new(MemoryVault::default());
        let keys = SshKeys::new(vault.clone(), state.clone());
        let secrets = Secrets::new(vault, state);

        let pool = Pool::builder()
            .key_vault(Arc::new(keys.clone()))
            .ssh_directory(home.path())
            .grace(grace)
            .build();
        let terminals = Terminals::new(journal.clone());
        let runbooks = Runbooks::new(projects.clone(), journal.clone(), terminals.clone());
        let mut registry = registry(&pool, &projects, &terminals, &secrets)?;
        RunbookActions::new(runbooks.clone()).register(&mut registry)?;

        let endpoint = endpoint(directory.path());
        let core = Arc::new(Core::new(registry, journal.clone(), Broker::new()));
        runbooks.connect(&core)?;
        let running = RunningServer::spawn(&endpoint, core)?;
        let client = Client::connect(&endpoint, app_hello(), &Autostart::off()).await?;
        let person = Person::approving_once(&client);

        let project = write_project(directory.path(), &servers)?;
        let opening = ActionCall::new("project.open").params(json!({ "path": project }));
        client.call(opening).await?;

        Ok(Some(Self {
            servers,
            _home: home,
            keys,
            client,
            journal,
            running,
            _person: person,
            _directory: directory,
        }))
    }
}

fn registry(
    pool: &Pool,
    projects: &Projects,
    terminals: &Terminals,
    secrets: &Secrets,
) -> Result<Registry, TestError> {
    let mut registry = Registry::new();
    let tunnels = Tunnels::new(pool.clone());

    registry.register(ProjectOpen::new(projects.clone()))?;
    registry.register(SshExec::new(pool.clone(), projects.clone()))?;
    registry.register(SshConnections::new(pool.clone()))?;
    registry.register(TunnelOpen::new(tunnels.clone(), projects.clone()))?;
    registry.register(TunnelClose::new(tunnels))?;
    registry.register(SshSessionOpen::new(
        pool.clone(),
        terminals.clone(),
        projects.clone(),
    ))?;
    registry.register(SessionBlocks::new(terminals.clone()))?;
    registry.register(SessionClose::new(terminals.clone()))?;
    registry.register(SessionList::new(terminals.clone()))?;
    SftpActions::new(pool.clone(), projects.clone()).register(&mut registry)?;
    registry.register(SecretPut::new(secrets.clone()))?;
    DbActions::new(pool.clone(), projects.clone(), secrets.clone()).register(&mut registry)?;

    Ok(registry)
}

fn app_hello() -> Hello {
    Hello::new(ClientKind::App, "synt-servers-tests")
}

fn journal(directory: &Path) -> Result<JournalWriter, TestError> {
    let keys = Arc::new(MemoryKeyStore::new()?);
    let journal = Journal::create(directory.join("journal.sqlite"), keys)?;

    Ok(JournalWriter::spawn(journal)?)
}

fn endpoint(directory: &Path) -> Endpoint {
    let unique = directory.file_name().unwrap_or_default().to_string_lossy();

    Endpoint::named(directory, &format!("synt-servers-{unique}"))
}

fn write_project(directory: &Path, servers: &SshServers) -> Result<PathBuf, TestError> {
    let project = directory.join(PROJECT);
    fs::create_dir_all(project.join("resources"))?;

    let synt_toml = format!("id = \"{PROJECT_ID}\"\nname = \"Lab\"\n");
    fs::write(project.join("synt.toml"), synt_toml)?;

    for (name, resource) in resources(servers)? {
        let text = toml::to_string(&resource)?;
        fs::write(project.join(format!("resources/{name}.toml")), text)?;
    }

    Ok(project)
}

fn resources(servers: &SshServers) -> Result<[(&'static str, Resource); 5], TestError> {
    let key = servers.client_key().display().to_string();
    let bastion = host(
        SshServers::BASTION_ADDRESS,
        Some(servers.bastion_port()),
        &key,
    );
    let inner = Value::Object(
        host(SshServers::TARGET_NAME, None, &key)
            .into_iter()
            .chain([("via".to_owned(), json!("bastion"))])
            .collect(),
    );
    let vaulted = Value::Object(
        bastion
            .clone()
            .into_iter()
            .chain([("identity".to_owned(), json!("secret://night"))])
            .collect(),
    );
    let echo = json!({
        "kind": "tunnel",
        "host": "inner",
        "remote": format!("127.0.0.1:{}", SshServers::ECHO_PORT),
    });

    Ok([
        ("bastion", resource(Value::Object(bastion))?),
        ("inner", resource(inner)?),
        ("vaulted", resource(vaulted)?),
        ("echo", resource(echo)?),
        ("postgres", resource(postgres())?),
    ])
}

fn postgres() -> Value {
    json!({
        "kind": "database",
        "engine": Engine::Postgres,
        "host": POSTGRES_HOST,
        "database": DbServers::DATABASE,
        "user": DbServers::USER,
        "password": format!("secret://{POSTGRES_PASSWORD}"),
        "via": "inner",
        "tls": TlsMode::Require,
    })
}

fn host(address: &str, port: Option<u16>, key: &str) -> Map<String, Value> {
    let mut fields = Map::new();

    fields.insert("kind".to_owned(), json!("host"));
    fields.insert("address".to_owned(), json!(address));
    fields.insert("user".to_owned(), json!(SshServers::USER));
    fields.insert("identity".to_owned(), json!(key));

    if let Some(port) = port {
        fields.insert("port".to_owned(), json!(port));
    }

    fields
}

fn resource(fields: Value) -> Result<Resource, TestError> {
    Ok(serde_json::from_value(fields)?)
}
