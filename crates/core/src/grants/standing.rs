use synt_journal::{Actor, ResourceRef};

use super::GrantId;
use super::actor_text::parse_actor;
use crate::access::AccessRequest;
use crate::state::StoredGrant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingGrant {
    pub id: GrantId,
    pub actor: Actor,
    pub action: String,
    pub resource: Option<String>,
    pub destination: Option<String>,
    pub created_at: u64,
}

impl StandingGrant {
    pub(super) fn from_stored(stored: StoredGrant) -> Option<Self> {
        Some(Self {
            id: GrantId(stored.id),
            actor: parse_actor(&stored.actor)?,
            action: stored.action,
            resource: stored.resource,
            destination: stored.destination,
            created_at: stored.created_at,
        })
    }

    pub fn covers(&self, request: &AccessRequest<'_>) -> bool {
        self.actor == *request.actor
            && self.action == request.action.as_str()
            && self.resource.as_deref() == request.target.map(ResourceRef::as_str)
            && self.destination.as_deref() == request.destination
    }
}
