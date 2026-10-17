use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_sync::{StatusReport, Syncer};

use super::params::ProjectParams;

pub struct SyncStatus {
    syncer: Syncer,
}

impl SyncStatus {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncStatus {
    const ID: &'static str = "sync.status";
    const TITLE: &'static str = "Show how a project syncs: members, key, conflicts";
    const RISK: Risk = Risk::Read;

    type Params = ProjectParams;
    type Output = StatusReport;

    fn summary(&self, output: &StatusReport) -> Value {
        json!({
            "project": output.project,
            "access": output.access,
            "documents": output.documents,
            "conflicts": output.conflicts.len(),
        })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ProjectParams,
    ) -> Result<StatusReport, ActionError> {
        Ok(self.syncer.status(&params.project).await?)
    }
}
