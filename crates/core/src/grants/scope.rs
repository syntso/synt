use synt_journal::{ActionId, Actor, ResourceRef};

use crate::access::AccessRequest;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GrantScope {
    actor: Actor,
    action: ActionId,
    target: Option<ResourceRef>,
    destination: Option<String>,
}

impl GrantScope {
    pub(crate) fn new(
        actor: Actor,
        action: ActionId,
        target: Option<ResourceRef>,
        destination: Option<String>,
    ) -> Self {
        Self {
            actor,
            action,
            target,
            destination,
        }
    }

    pub(crate) fn covering(request: &AccessRequest<'_>) -> Self {
        Self::new(
            request.actor.clone(),
            request.action.clone(),
            request.target.cloned(),
            request.destination.map(str::to_owned),
        )
    }

    pub(crate) fn actor(&self) -> &Actor {
        &self.actor
    }

    pub(crate) fn action(&self) -> &ActionId {
        &self.action
    }

    pub(crate) fn target(&self) -> Option<&ResourceRef> {
        self.target.as_ref()
    }

    pub(crate) fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    pub(crate) fn covers(&self, request: &AccessRequest<'_>) -> bool {
        self.actor == *request.actor
            && self.action == *request.action
            && self.target.as_ref() == request.target
            && self.destination.as_deref() == request.destination
    }
}
