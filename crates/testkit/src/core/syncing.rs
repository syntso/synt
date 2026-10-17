use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, ActionGroup, Registry};
use synt_core::{Broker, Core, ProjectOpen, Projects, ResourceGet, ResourcePut, SyncActions};
use synt_proto::ActionCall;
use synt_sync::{SyncLoop, Syncer};

use super::{Grounds, Harness};
use crate::TestError;

pub const PROJECT_ID: &str = "4f2c9a7e1b3d5f60718293a4b5c6d7e8";
const PROJECT_FILE: &str = "id = \"4f2c9a7e1b3d5f60718293a4b5c6d7e8\"\nname = \"Orders\"\n";

pub fn register_sync(registry: &mut Registry, syncer: &Syncer) -> Result<(), ActionError> {
    SyncActions::new(syncer.clone()).register(registry)
}

pub struct SyncingCore {
    harness: Harness,
    syncer: Syncer,
    project: PathBuf,
    shared: PathBuf,
    _every: Option<SyncLoop>,
}

impl SyncingCore {
    pub fn start(shared: &Path, every: Option<Duration>) -> Result<Self, TestError> {
        let grounds = Grounds::new()?;
        let project = grounds.path().join("orders");
        fs::create_dir_all(project.join("resources"))?;
        fs::write(project.join("synt.toml"), PROJECT_FILE)?;

        let projects = Projects::start(grounds.state()?)?;
        let syncer = Syncer::builder(
            &grounds.state_path(),
            grounds.vault(),
            Arc::new(projects.clone()),
        )
        .journal(grounds.journal())
        .open()?;

        let mut registry = grounds.registry()?;
        registry.register(ProjectOpen::new(projects.clone()))?;
        registry.register(ResourcePut::new(projects.clone()))?;
        registry.register(ResourceGet::new(projects))?;
        register_sync(&mut registry, &syncer)?;

        let core = Core::new(registry, grounds.journal(), Broker::new());

        Ok(Self {
            harness: Harness::serve(grounds, core)?,
            _every: every.map(|every| syncer.start(every)),
            syncer,
            project,
            shared: shared.to_owned(),
        })
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

    pub async fn call(&self, action: &str, params: Value) -> Result<Value, TestError> {
        self.send(ActionCall::new(action).params(params)).await
    }

    pub async fn send(&self, call: ActionCall) -> Result<Value, TestError> {
        let app = self.harness.app().await?;
        let action = call.action.clone();

        let outcome = app
            .call(call)
            .await
            .map_err(|error| format!("{action} failed: {error}"))?;

        Ok(outcome.value)
    }

    pub async fn open_and_enable(&self) -> Result<Value, TestError> {
        self.call(ProjectOpen::ID, json!({ "path": self.project }))
            .await?;

        self.call(
            "sync.enable",
            json!({ "project": "orders", "folder": self.shared }),
        )
        .await
    }

    pub async fn device(&self) -> Result<String, TestError> {
        let status = self
            .call("sync.status", json!({ "project": "orders" }))
            .await?;
        let device = status["device"]
            .as_str()
            .ok_or("the status names no device")?;

        Ok(device.to_owned())
    }

    pub async fn share_with(&self, other: &SyncingCore) -> Result<(), TestError> {
        let member = other.device().await?;

        self.call(
            "sync.share",
            json!({ "project": "orders", "member": member }),
        )
        .await?;

        Ok(())
    }

    pub async fn sync(&self) {
        for report in self.syncer.sync_now().await {
            assert_eq!(report.error, None, "a sync round failed");
        }
    }

    pub async fn put_host(&self, name: &str, address: &str, user: &str) -> Result<(), TestError> {
        let call = ActionCall::new(ResourcePut::ID)
            .target(format!("@orders/{name}"))
            .params(json!({ "kind": "host", "address": address, "user": user }));

        self.send(call).await?;

        Ok(())
    }

    pub async fn host(&self, name: &str) -> Result<Option<Value>, TestError> {
        let app = self.harness.app().await?;
        let call =
            ActionCall::new(ResourceGet::ID).params(json!({ "project": "orders", "name": name }));

        Ok(app.call(call).await.ok().map(|outcome| outcome.value))
    }
}
