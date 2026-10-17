use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use synt_actions::{Action, Registry};
use synt_core::{Broker, Core, ProjectOpen, Projects, ResourceGet, ResourcePut, SyncActions};
use synt_proto::ActionCall;
use synt_sync::{SyncLoop, Syncer};

use super::{Grounds, Harness};

pub const PROJECT_ID: &str = "4f2c9a7e1b3d5f60718293a4b5c6d7e8";
const PROJECT_FILE: &str = "id = \"4f2c9a7e1b3d5f60718293a4b5c6d7e8\"\nname = \"Orders\"\n";

pub fn register_sync(registry: &mut Registry, syncer: &Syncer) {
    SyncActions::new(syncer.clone()).register(registry).unwrap();
}

pub struct SyncingCore {
    harness: Harness,
    syncer: Syncer,
    project: PathBuf,
    shared: PathBuf,
    _every: Option<SyncLoop>,
}

impl SyncingCore {
    pub fn start(shared: &Path, every: Option<Duration>) -> Self {
        let grounds = Grounds::new();
        let project = grounds.path().join("orders");
        fs::create_dir_all(project.join("resources")).unwrap();
        fs::write(project.join("synt.toml"), PROJECT_FILE).unwrap();

        let projects = Projects::start(grounds.state()).unwrap();
        let syncer = Syncer::builder(
            &grounds.state_path(),
            grounds.vault(),
            Arc::new(projects.clone()),
        )
        .journal(grounds.journal())
        .open()
        .unwrap();

        let mut registry = grounds.registry();
        registry
            .register(ProjectOpen::new(projects.clone()))
            .unwrap();
        registry
            .register(ResourcePut::new(projects.clone()))
            .unwrap();
        registry.register(ResourceGet::new(projects)).unwrap();
        register_sync(&mut registry, &syncer);

        let core = Core::new(registry, grounds.journal(), Broker::new());

        Self {
            harness: Harness::serve(grounds, core),
            _every: every.map(|every| syncer.start(every)),
            syncer,
            project,
            shared: shared.to_owned(),
        }
    }

    pub fn harness(&self) -> &Harness {
        &self.harness
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    pub fn vault_items(&self) -> Vec<String> {
        self.harness.grounds().vault_items()
    }

    pub async fn call(&self, action: &str, params: Value) -> Value {
        self.send(ActionCall::new(action).params(params)).await
    }

    pub async fn send(&self, call: ActionCall) -> Value {
        let app = self.harness.app().await;
        let action = call.action.clone();

        app.call(call)
            .await
            .unwrap_or_else(|error| panic!("{action} failed: {error}"))
            .value
    }

    pub async fn open_and_enable(&self) -> Value {
        self.call(ProjectOpen::ID, json!({ "path": self.project }))
            .await;

        self.call(
            "sync.enable",
            json!({ "project": "orders", "folder": self.shared }),
        )
        .await
    }

    pub async fn device(&self) -> String {
        let status = self
            .call("sync.status", json!({ "project": "orders" }))
            .await;

        status["device"].as_str().unwrap().to_owned()
    }

    pub async fn share_with(&self, other: &SyncingCore) {
        let member = other.device().await;

        self.call(
            "sync.share",
            json!({ "project": "orders", "member": member }),
        )
        .await;
    }

    pub async fn sync(&self) {
        for report in self.syncer.sync_now().await {
            assert_eq!(report.error, None, "a sync round failed");
        }
    }

    pub async fn put_host(&self, name: &str, address: &str, user: &str) {
        let call = ActionCall::new(ResourcePut::ID)
            .target(format!("@orders/{name}"))
            .params(json!({ "kind": "host", "address": address, "user": user }));

        self.send(call).await;
    }

    pub async fn host(&self, name: &str) -> Option<Value> {
        let app = self.harness.app().await;
        let call =
            ActionCall::new(ResourceGet::ID).params(json!({ "project": "orders", "name": name }));

        app.call(call).await.ok().map(|outcome| outcome.value)
    }
}
