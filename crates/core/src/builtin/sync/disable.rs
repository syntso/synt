use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_sync::{Disabled, Syncer};

use super::params::ProjectParams;

pub struct SyncDisable {
    syncer: Syncer,
}

impl SyncDisable {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncDisable {
    const ID: &'static str = "sync.disable";
    const TITLE: &'static str = "Stop syncing a project on this device";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = ProjectParams;
    type Output = Disabled;

    fn summary(&self, output: &Disabled) -> Value {
        json!({ "project": output.project, "left": output.left })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ProjectParams,
    ) -> Result<Disabled, ActionError> {
        Ok(self.syncer.disable(&params.project).await?)
    }
}
