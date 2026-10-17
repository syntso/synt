use serde::Serialize;
use synt_journal::Actor;

use crate::grants::{GrantId, StandingGrant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrantInfo {
    pub id: GrantId,
    pub actor: Actor,
    pub action: String,
    pub resource: Option<String>,
    pub destination: Option<String>,
    pub scope: &'static str,
    pub created_at: u64,
}

impl From<StandingGrant> for GrantInfo {
    fn from(grant: StandingGrant) -> Self {
        Self {
            id: grant.id,
            actor: grant.actor,
            action: grant.action,
            resource: grant.resource,
            destination: grant.destination,
            scope: "always",
            created_at: grant.created_at,
        }
    }
}
