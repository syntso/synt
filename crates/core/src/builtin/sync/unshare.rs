use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_sync::{MembershipChange, Syncer};

use super::params::MemberParams;

pub struct SyncUnshare {
    syncer: Syncer,
}

impl SyncUnshare {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncUnshare {
    const ID: &'static str = "sync.unshare";
    const TITLE: &'static str = "Remove a member from a project and rotate its key";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = MemberParams;
    type Output = MembershipChange;

    fn summary(&self, output: &MembershipChange) -> Value {
        json!({
            "project": output.project,
            "member": output.member,
            "generation": output.generation,
        })
    }

    async fn run(
        &self,
        _context: &Context,
        params: MemberParams,
    ) -> Result<MembershipChange, ActionError> {
        Ok(self
            .syncer
            .unshare(params.project.as_str(), params.member)
            .await?)
    }
}
