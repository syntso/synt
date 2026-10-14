use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use synt_client::{Client, ClientError};
use synt_core::{Broker, Core, ProjectOpen, Projects, Runbooks, Terminals};
use synt_proto::{ActionCall, ActionOutcome, Notification, RunbookStep};
use tokio::sync::broadcast;

use super::{ExecStub, Grounds, Harness, eventually};

const PROJECT: &str = "ops";
const PROJECT_ID: &str = "0123456789abcdef0123456789abcdef";

pub struct RunbookDesk {
    harness: Harness,
    folder: PathBuf,
}

impl RunbookDesk {
    pub const PROJECT: &'static str = PROJECT;
    pub const PROJECT_ID: &'static str = PROJECT_ID;

    pub async fn start() -> Self {
        let grounds = Grounds::new();
        let projects = Projects::start(grounds.state()).unwrap();
        let runbooks = Runbooks::new(
            projects.clone(),
            grounds.journal(),
            Terminals::new(grounds.journal()),
        );

        let mut registry = grounds.registry();
        registry.register(ExecStub).unwrap();
        registry.register(ProjectOpen::new(projects)).unwrap();
        runbooks.register(&mut registry).unwrap();

        let core = Arc::new(Core::new(registry, grounds.journal(), Broker::new()));
        runbooks.connect(&core);

        let folder = grounds.path().join(PROJECT);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("synt.toml"), format!("id = \"{PROJECT_ID}\"\n")).unwrap();

        let harness = Harness::serve(grounds, core);
        let opening = ActionCall::new("project.open").params(json!({ "path": folder }));
        harness.app().await.call(opening).await.unwrap();

        Self { harness, folder }
    }

    pub fn harness(&self) -> &Harness {
        &self.harness
    }

    pub fn write(&self, name: &str, text: &str) {
        let folder = self.folder.join("runbooks");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join(format!("{name}.toml")), text).unwrap();
    }

    pub fn file(&self, name: &str) -> String {
        fs::read_to_string(self.folder.join(format!("runbooks/{name}.toml"))).unwrap()
    }

    pub fn reference(name: &str) -> String {
        format!("@{PROJECT_ID}/{name}")
    }

    pub fn run_call(name: &str, params: &Value, approve_steps: bool) -> ActionCall {
        ActionCall::new("runbook.run").params(json!({
            "project": PROJECT,
            "name": name,
            "params": params,
            "approve_steps": approve_steps,
        }))
    }

    pub async fn run(
        &self,
        client: &Client,
        name: &str,
        params: &Value,
        approve_steps: bool,
    ) -> Result<ActionOutcome, ClientError> {
        client
            .call(Self::run_call(name, params, approve_steps))
            .await
    }

    pub async fn hear(
        notifications: &mut broadcast::Receiver<Notification>,
        count: usize,
    ) -> Vec<RunbookStep> {
        let mut heard = Vec::new();

        eventually("the run reports its steps", async || {
            heard.extend(Self::steps_heard(notifications));
            (heard.len() >= count).then_some(())
        })
        .await;

        heard
    }

    pub fn steps_heard(notifications: &mut broadcast::Receiver<Notification>) -> Vec<RunbookStep> {
        let mut heard = Vec::new();

        while let Ok(notification) = notifications.try_recv() {
            if let Some(step) = notification.read::<RunbookStep>() {
                heard.push(step.unwrap());
            }
        }

        heard
    }
}
