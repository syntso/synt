use synt_journal::{ActionId, Actor, ResourceRef};

use crate::access::AccessRequest;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Grant {
    actor: Actor,
    action: ActionId,
    target: Option<ResourceRef>,
}

impl Grant {
    pub(super) fn covering(request: &AccessRequest<'_>) -> Self {
        Self {
            actor: request.actor.clone(),
            action: request.action.clone(),
            target: request.target.cloned(),
        }
    }
}
