use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use synt_actions::Registry;
use synt_client::{Autostart, Client, ClientError};
use synt_core::{
    Broker, Core, Files, HostResource, ProjectOpen, Projects, Resource, Runbooks, SessionBlocks,
    SessionClose, SessionList, SshConnections, SshExec, SshKeys, SshSessionOpen, State, Terminals,
    TunnelClose, TunnelOpen, TunnelResource,
};
use synt_journal::{Event, EventKind, Filter, Journal, JournalWriter, MemoryKeyStore, Seq};
use synt_platform::Endpoint;
use synt_proto::{ActionCall, ActionOutcome, ClientKind, Hello, RequestSeq};
use synt_ssh::{Pool, Tunnels};
use synt_test_servers::{SshHome, SshServers};
use tempfile::TempDir;
use tokio::time;

use crate::{MemoryVault, Person, RunningCore};

pub const PROJECT: &str = "lab";
pub const BASTION: &str = "@lab/bastion";
pub const INNER: &str = "@lab/inner";
pub const VAULTED: &str = "@lab/vaulted";
pub const ECHO: &str = "@lab/echo";

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
    running: RunningCore,
    _person: Person,
    _directory: TempDir,
}

impl Lab {
    pub async fn start() -> Option<Self> {
        Self::build(SshServers::trusting_home, DEFAULT_GRACE).await
    }

    pub async fn knowing(home: impl FnOnce(&SshServers) -> SshHome) -> Option<Self> {
        Self::build(home, DEFAULT_GRACE).await
    }

    pub async fn with_grace(grace: Duration) -> Option<Self> {
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

    pub async fn client_port(&self, host: &str) -> String {
        let outcome = self.exec(host, "printenv SSH_CLIENT").await.unwrap();
        let line = outcome.value["stdout"].as_str().unwrap().to_owned();

        line.split_whitespace().nth(1).unwrap().to_owned()
    }

    pub fn trail(&self, request: RequestSeq) -> Vec<Event> {
        self.events(&Filter::all().call(Seq::new(request.0)))
    }

    pub fn kinds(&self, request: RequestSeq) -> Vec<EventKind> {
        self.trail(request).iter().map(Event::kind).collect()
    }

    pub async fn finished(&self, request: RequestSeq) -> Value {
        self.settled(request, EventKind::Finished).await
    }

    pub fn last_failure(&self, action: &str) -> Value {
        self.last_of(action, EventKind::Failed)
    }

    pub fn last_approval(&self, action: &str) -> Value {
        self.last_of(action, EventKind::Approved)
    }

    pub async fn stop(self) {
        let Self {
            client,
            running,
            _person: person,
            ..
        } = self;

        drop(person);
        drop(client);

        running.stop().await;
    }

    async fn settled(&self, request: RequestSeq, kind: EventKind) -> Value {
        time::timeout(WAIT, async {
            loop {
                let found = self
                    .trail(request)
                    .into_iter()
                    .find(|event| event.kind() == kind);

                if let Some(event) = found {
                    return event.details().value().clone();
                }

                time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("the journal never recorded {kind:?} for {request:?}"))
    }

    fn last_of(&self, action: &str, kind: EventKind) -> Value {
        let events = self.events(&Filter::all());

        let last = events
            .iter()
            .rev()
            .find(|event| event.kind() == kind && event.action().as_str() == action)
            .unwrap_or_else(|| panic!("the journal holds no {kind:?} event of {action}"));

        last.details().value().clone()
    }

    fn events(&self, filter: &Filter) -> Vec<Event> {
        let reader = self.journal.reader().unwrap();

        reader.page(filter, None, PAGE).unwrap()
    }

    async fn build(home: impl FnOnce(&SshServers) -> SshHome, grace: Duration) -> Option<Self> {
        let servers = SshServers::running()?;
        let home = home(&servers);
        let directory = tempfile::tempdir().unwrap();

        let journal = journal(directory.path());
        let state = State::open(&directory.path().join("state.sqlite")).unwrap();
        let projects = Projects::start(state.clone()).unwrap();
        let keys = SshKeys::new(Arc::new(MemoryVault::default()), state);

        let pool = Pool::builder()
            .key_vault(Arc::new(keys.clone()))
            .ssh_directory(home.path())
            .grace(grace)
            .build();
        let terminals = Terminals::new(journal.clone());
        let runbooks = Runbooks::new(projects.clone(), journal.clone(), terminals.clone());
        let mut registry = registry(&pool, &projects, &terminals);
        runbooks.register(&mut registry).unwrap();

        let endpoint = endpoint(directory.path());
        let core = Arc::new(Core::new(registry, journal.clone(), Broker::new()));
        runbooks.connect(&core);
        let running = RunningCore::spawn(&endpoint, core);
        let client = Client::connect(&endpoint, app_hello(), &Autostart::off())
            .await
            .unwrap();
        let person = Person::approving_once(&client);

        let project = write_project(directory.path(), &servers);
        let opening = ActionCall::new("project.open").params(json!({ "path": project }));
        client.call(opening).await.unwrap();

        Some(Self {
            servers,
            _home: home,
            keys,
            client,
            journal,
            running,
            _person: person,
            _directory: directory,
        })
    }
}

fn registry(pool: &Pool, projects: &Projects, terminals: &Terminals) -> Registry {
    let mut registry = Registry::new();
    let tunnels = Tunnels::new(pool.clone());

    registry
        .register(ProjectOpen::new(projects.clone()))
        .unwrap();
    registry
        .register(SshExec::new(pool.clone(), projects.clone()))
        .unwrap();
    registry
        .register(SshConnections::new(pool.clone()))
        .unwrap();
    registry
        .register(TunnelOpen::new(tunnels.clone(), projects.clone()))
        .unwrap();
    registry.register(TunnelClose::new(tunnels)).unwrap();
    registry
        .register(SshSessionOpen::new(
            pool.clone(),
            terminals.clone(),
            projects.clone(),
        ))
        .unwrap();
    registry
        .register(SessionBlocks::new(terminals.clone()))
        .unwrap();
    registry
        .register(SessionClose::new(terminals.clone()))
        .unwrap();
    registry
        .register(SessionList::new(terminals.clone()))
        .unwrap();
    Files::new(pool.clone(), projects.clone())
        .register(&mut registry)
        .unwrap();

    registry
}

fn app_hello() -> Hello {
    Hello::new(ClientKind::App, "synt-servers-tests")
}

fn journal(directory: &Path) -> JournalWriter {
    let keys = Arc::new(MemoryKeyStore::new().unwrap());
    let journal = Journal::create(directory.join("journal.sqlite"), keys).unwrap();

    JournalWriter::spawn(journal).unwrap()
}

fn endpoint(directory: &Path) -> Endpoint {
    let unique = directory.file_name().unwrap().to_string_lossy();

    Endpoint::named(directory, &format!("synt-servers-{unique}"))
}

fn write_project(directory: &Path, servers: &SshServers) -> PathBuf {
    let project = directory.join(PROJECT);
    fs::create_dir_all(project.join("resources")).unwrap();

    let synt_toml = format!("id = \"{PROJECT_ID}\"\nname = \"Lab\"\n");
    fs::write(project.join("synt.toml"), synt_toml).unwrap();

    for (name, resource) in resources(servers) {
        let text = toml::to_string(&resource).unwrap();
        fs::write(project.join(format!("resources/{name}.toml")), text).unwrap();
    }

    project
}

fn resources(servers: &SshServers) -> [(&'static str, Resource); 4] {
    let key = servers.client_key().display().to_string();
    let bastion = host(
        SshServers::BASTION_ADDRESS,
        Some(servers.bastion_port()),
        &key,
    );
    let echo = TunnelResource {
        host: "inner".to_owned(),
        remote: format!("127.0.0.1:{}", SshServers::ECHO_PORT),
        local: None,
    };

    [
        ("bastion", Resource::Host(bastion.clone())),
        (
            "inner",
            Resource::Host(HostResource {
                via: Some("bastion".to_owned()),
                ..host(SshServers::TARGET_NAME, None, &key)
            }),
        ),
        (
            "vaulted",
            Resource::Host(HostResource {
                identity: Some("secret://night".to_owned()),
                ..bastion
            }),
        ),
        ("echo", Resource::Tunnel(echo)),
    ]
}

fn host(address: &str, port: Option<u16>, key: &str) -> HostResource {
    HostResource {
        address: address.to_owned(),
        port,
        user: Some(SshServers::USER.to_owned()),
        via: None,
        identity: Some(key.to_owned()),
    }
}
