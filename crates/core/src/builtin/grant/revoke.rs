use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::{ActionId, Details, EventKind, JournalWriter, NewEvent};

use super::GrantInfo;
use crate::grants::{AlwaysGrants, GrantId};

#[derive(Clone)]
pub struct GrantRevoke {
    grants: AlwaysGrants,
    journal: JournalWriter,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct RevokeParams {
    pub id: GrantId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Revoked {
    pub grant: GrantInfo,
    pub stopped: usize,
}

impl GrantRevoke {
    pub fn new(grants: AlwaysGrants, journal: JournalWriter) -> Self {
        Self { grants, journal }
    }

    async fn note(&self, context: &Context, grant: &GrantInfo) -> Result<(), ActionError> {
        let details = json!({
            "revoked": grant.id,
            "actor": grant.actor,
            "action": grant.action,
            "resource": grant.resource,
            "destination": grant.destination,
        });
        let details = Details::from_serialize(&details)
            .map_err(|error| ActionError::failed(error.to_string()))?;
        let action =
            ActionId::new(Self::ID).map_err(|error| ActionError::failed(error.to_string()))?;

        let note = NewEvent::new(context.actor().clone(), EventKind::Note, action)
            .call(context.request())
            .details(details);

        self.journal
            .append_async(note)
            .await
            .map(|_| ())
            .map_err(|error| {
                ActionError::failed(format!("the revocation was not journaled: {error}"))
            })
    }
}

impl Action for GrantRevoke {
    const ID: &'static str = "grant.revoke";
    const TITLE: &'static str = "Revoke a standing grant and stop the calls that use it";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = RevokeParams;
    type Output = Revoked;

    fn summary(&self, output: &Revoked) -> Value {
        json!({ "revoked": output.grant.id, "stopped": output.stopped })
    }

    async fn run(&self, context: &Context, params: RevokeParams) -> Result<Revoked, ActionError> {
        let revoked = self
            .grants
            .revoke(params.id)
            .await
            .map_err(|error| ActionError::Failed(error.to_string()))?;

        let Some((grant, stopped)) = revoked else {
            return Err(ActionError::failed(format!(
                "there is no grant {}",
                params.id
            )));
        };

        let grant = GrantInfo::from(grant);
        self.note(context, &grant).await?;

        Ok(Revoked { grant, stopped })
    }
}
