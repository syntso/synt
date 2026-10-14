use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::ProjectParams;
use super::failure::failure;
use crate::Syncer;
use crate::report::SyncStatus;

pub(crate) struct SyncStatusAction {
    syncer: Syncer,
}

impl SyncStatusAction {
    pub(crate) fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncStatusAction {
    const ID: &'static str = "sync.status";
    const TITLE: &'static str = "Show how a project syncs: members, key, conflicts";
    const RISK: Risk = Risk::Read;

    type Params = ProjectParams;
    type Output = SyncStatus;

    fn summary(&self, output: &SyncStatus) -> Value {
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
    ) -> Result<SyncStatus, ActionError> {
        self.syncer.status(&params.project).await.map_err(failure)
    }
}
