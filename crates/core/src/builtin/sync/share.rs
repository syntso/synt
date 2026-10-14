use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_sync::{MembershipChange, Syncer};

use super::params::MemberParams;

pub struct SyncShare {
    syncer: Syncer,
}

impl SyncShare {
    pub fn new(syncer: Syncer) -> Self {
        Self { syncer }
    }
}

impl Action for SyncShare {
    const ID: &'static str = "sync.share";
    const TITLE: &'static str = "Share a project's key with a member who asked to join";
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
        Ok(self.syncer.share(&params.project, params.member).await?)
    }
}
