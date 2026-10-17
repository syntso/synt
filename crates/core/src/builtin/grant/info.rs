use serde::Serialize;
use synt_journal::{Actor, ResourceRef, Timestamp};

use crate::grants::{GrantId, StandingGrant};

const SCOPE: &str = "always";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GrantInfo {
    pub id: GrantId,
    pub actor: Actor,
    pub action: String,
    pub resource: Option<String>,
    pub destination: Option<String>,
    pub scope: &'static str,
    pub created_at: Timestamp,
}

impl From<StandingGrant> for GrantInfo {
    fn from(grant: StandingGrant) -> Self {
        let scope = grant.scope();

        Self {
            id: grant.id(),
            actor: scope.actor().clone(),
            action: scope.action().as_str().to_owned(),
            resource: scope.target().map(ResourceRef::as_str).map(str::to_owned),
            destination: scope.destination().map(str::to_owned),
            scope: SCOPE,
            created_at: grant.created_at(),
        }
    }
}
