use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use synt_actions::Registry;
use synt_client::{Autostart, Client, ClientError};
use synt_core::{
    Broker, ClusterResource, Core, HostResource, Kube, ProjectOpen, Projects, Resource,
    ResourceList, State,
};
use synt_journal::{Event, EventKind, Filter, Journal, JournalWriter, MemoryKeyStore, Seq};
use synt_kube::KubeconfigSource;
use synt_platform::Endpoint;
use synt_proto::{
    ActionCall, ActionOutcome, ApprovalDecision, ApprovalRequest, ClientKind, Hello, RequestSeq,
};
use synt_ssh::Pool;
use synt_test_servers::{KubeServers, SshHome};
use synt_testkit::TestError;
use synt_testkit::core::RunningServer;
use tempfile::TempDir;
use tokio::time;

use crate::Desk;

pub const DIRECT: &str = "@lab/direct";
pub const INSIDE: &str = "@lab/inside";

const PROJECT_ID: &str = "c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1";
const PAGE: u32 = 10_000;
const WAIT: Duration = Duration::from_secs(30);

pub struct Lab {
    pub app: Client,
    pub agent: Client,
    pub desk: Desk,
    pub namespace: String,
    journal: JournalWriter,
    running: RunningServer,
    _home: SshHome,
    _directory: TempDir,
}

impl Lab {
    pub async fn start() -> Result<Option<Self>, TestError> {
        let Some(servers) = KubeServers::running() else {
            return Ok(None);
        };
        let home = servers.trusting_home();
        let directory = tempfile::tempdir()?;
        let namespace = unique_namespace();

        let journal = journal(directory.path())?;
        let state = State::open(&directory.path().join("state.sqlite"))?;
        let personal = KubeconfigSource::File(servers.kubeconfig());
        let projects = Projects::start_with(state, personal)?;
        let pool = Pool::builder().ssh_directory(home.path()).build();

        let mut registry = Registry::new();
        registry.register(ProjectOpen::new(projects.clone()))?;
        registry.register(ResourceList::new(projects.clone()))?;
        Kube::new(pool, projects, journal.clone()).register(&mut registry)?;

        let endpoint = endpoint(directory.path());
        let core = Core::new(registry, journal.clone(), Broker::new());
        let running = RunningServer::spawn(&endpoint, core)?;
        let app = connect(&endpoint, ClientKind::App).await?;
        let agent = connect(
            &endpoint,
            ClientKind::Agent {
                name: "tester".to_owned(),
            },
        )
        .await?;
        let desk = Desk::new(&app);

        let project = write_project(directory.path(), &servers, &namespace)?;
        let opening = ActionCall::new("project.open").params(json!({ "path": project }));
        app.call(opening).await?;

        let lab = Self {
            app,
            agent,
            desk,
            namespace,
            journal,
            running,
            _home: home,
            _directory: directory,
        };
        lab.person(
            DIRECT,
            "k8s.apply",
            json!({ "manifest": lab.namespace_manifest() }),
        )
        .await?;

        Ok(Some(lab))
    }

    pub async fn person(
        &self,
        target: &str,
        action: &str,
        params: Value,
    ) -> Result<ActionOutcome, ClientError> {
        self.app.call(call(target, action, params)).await
    }

    pub async fn agent(
        &self,
        target: &str,
        action: &str,
        params: Value,
    ) -> Result<ActionOutcome, ClientError> {
        self.agent.call(call(target, action, params)).await
    }

    pub async fn agent_asking(
        &self,
        target: &str,
        action: &str,
        params: Value,
    ) -> Result<(ApprovalRequest, Result<ActionOutcome, ClientError>), TestError> {
        self.desk
            .decide(
                &self.agent,
                call(target, action, params),
                ApprovalDecision::AllowOnce,
            )
            .await
    }

    pub async fn person_asking(
        &self,
        target: &str,
        action: &str,
        params: Value,
    ) -> Result<(ApprovalRequest, Result<ActionOutcome, ClientError>), TestError> {
        self.desk
            .decide(
                &self.app,
                call(target, action, params),
                ApprovalDecision::AllowOnce,
            )
            .await
    }

    pub fn requested(&self, request: RequestSeq) -> Value {
        let Some(detail) = self.detail_of(request, EventKind::Requested) else {
            panic!("every call is requested");
        };
        detail
    }

    pub async fn finished(&self, request: RequestSeq) -> Value {
        time::timeout(WAIT, async {
            loop {
                if let Some(details) = self.detail_of(request, EventKind::Finished) {
                    return details;
                }

                time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("the journal never recorded Finished for {request:?}"))
    }

    pub fn journal_text(&self) -> Result<String, TestError> {
        Ok(self
            .events(&Filter::all())?
            .iter()
            .map(|event| event.details().value().to_string())
            .collect::<Vec<_>>()
            .join("\n"))
    }

    pub async fn finish(self) -> Result<(), TestError> {
        let namespace = self.namespace.clone();
        let params = json!({ "kind": "namespace", "name": namespace });
        let (_, deleted) = self.person_asking(DIRECT, "k8s.delete", params).await?;
        deleted?;

        let Self {
            app,
            agent,
            running,
            ..
        } = self;
        drop(app);
        drop(agent);

        running.stop().await?;

        Ok(())
    }

    fn namespace_manifest(&self) -> String {
        format!(
            "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {}\n",
            self.namespace
        )
    }

    fn detail_of(&self, request: RequestSeq, kind: EventKind) -> Option<Value> {
        self.events(&Filter::all().call(Seq::new(request.get())))
            .ok()?
            .into_iter()
            .find(|event| event.kind() == kind)
            .map(|event| event.details().value().clone())
    }

    fn events(&self, filter: &Filter) -> Result<Vec<Event>, TestError> {
        let reader = self.journal.reader()?;

        Ok(reader.page(filter, None, PAGE)?)
    }
}

pub fn call(target: &str, action: &str, params: Value) -> ActionCall {
    let call = ActionCall::new(action).params(params);

    if target.is_empty() {
        return call;
    }

    call.target(target)
}

pub fn ticker(name: &str) -> String {
    format!(
        "apiVersion: apps/v1
kind: Deployment
metadata:
  name: {name}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: {name}
  template:
    metadata:
      labels:
        app: {name}
    spec:
      terminationGracePeriodSeconds: 1
      containers:
      - name: main
        image: busybox:1.37
        ports:
        - name: web
          containerPort: 8080
        command:
        - sh
        - -c
        - mkdir -p /www; echo hello from {name} > /www/index.html; httpd -p 8080 -h /www; i=0; while true; do echo tick $i; i=$((i+1)); sleep 1; done
---
apiVersion: v1
kind: Service
metadata:
  name: {name}
spec:
  selector:
    app: {name}
  ports:
  - port: 80
    targetPort: web
"
    )
}

async fn connect(endpoint: &Endpoint, kind: ClientKind) -> Result<Client, TestError> {
    Ok(Client::connect(
        endpoint,
        Hello::new(kind, "synt-cluster-tests"),
        &Autostart::off(),
    )
    .await?)
}

fn journal(directory: &Path) -> Result<JournalWriter, TestError> {
    let keys = Arc::new(MemoryKeyStore::new()?);
    let journal = Journal::create(directory.join("journal.sqlite"), keys)?;

    Ok(JournalWriter::spawn(journal)?)
}

fn endpoint(directory: &Path) -> Endpoint {
    let unique = directory.file_name().unwrap_or_default().to_string_lossy();

    Endpoint::named(directory, &format!("synt-clusters-{unique}"))
}

fn write_project(
    directory: &Path,
    servers: &KubeServers,
    namespace: &str,
) -> Result<PathBuf, TestError> {
    let project = directory.join("lab");
    fs::create_dir_all(project.join("resources"))?;
    fs::write(
        project.join("synt.toml"),
        format!("id = \"{PROJECT_ID}\"\nname = \"Lab\"\n"),
    )?;

    for (name, resource) in resources(servers, namespace) {
        let text = toml::to_string(&resource)?;
        fs::write(project.join(format!("resources/{name}.toml")), text)?;
    }

    Ok(project)
}

fn resources(servers: &KubeServers, namespace: &str) -> [(&'static str, Resource); 3] {
    let kubeconfig = servers.kubeconfig().display().to_string();
    let cluster = |context: &str, via: Option<&str>| ClusterResource {
        kubeconfig: Some(kubeconfig.clone()),
        context: Some(context.to_owned()),
        namespace: Some(namespace.to_owned()),
        via: via.map(str::to_owned),
    };
    let bastion = HostResource {
        address: KubeServers::BASTION_ADDRESS.to_owned(),
        port: Some(servers.bastion_port()),
        user: Some(KubeServers::USER.to_owned()),
        via: None,
        identity: Some(servers.client_key().display().to_string()),
    };

    [
        ("bastion", Resource::Host(bastion)),
        (
            "direct",
            Resource::Cluster(cluster(KubeServers::DIRECT, None)),
        ),
        (
            "inside",
            Resource::Cluster(cluster(KubeServers::INSIDE, Some("bastion"))),
        ),
    ]
}

fn unique_namespace() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or_default();

    format!("synt-core-{}-{nanos}", process::id())
}
