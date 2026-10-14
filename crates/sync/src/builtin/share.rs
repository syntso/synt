use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::params::MemberParams;
use crate::Syncer;
use crate::id::MemberId;
use crate::report::MembershipChange;

pub(crate) struct SyncShare {
    syncer: Syncer,
}

impl SyncShare {
    pub(crate) fn new(syncer: Syncer) -> Self {
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
        let member = MemberId::parse(&params.member)?;

        Ok(self.syncer.share(&params.project, member).await?)
    }
}
