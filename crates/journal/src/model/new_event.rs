use super::{ActionId, Actor, EventKind, ResourceRef, Seq};
use crate::payload::Details;

#[derive(Debug, Clone, PartialEq)]
pub struct NewEvent {
    actor: Actor,
    actor_bytes: Vec<u8>,
    kind: EventKind,
    call: Option<Seq>,
    action: ActionId,
    resource: Option<ResourceRef>,
    details: Details,
}

impl NewEvent {
    pub fn new(actor: Actor, kind: EventKind, action: ActionId) -> Self {
        let actor_bytes = actor.encode();

        Self {
            actor,
            actor_bytes,
            kind,
            call: None,
            action,
            resource: None,
            details: Details::empty(),
        }
    }

    #[must_use]
    pub fn call(mut self, request: Seq) -> Self {
        self.call = Some(request);
        self
    }

    #[must_use]
    pub fn resource(mut self, resource: ResourceRef) -> Self {
        self.resource = Some(resource);
        self
    }

    #[must_use]
    pub fn details(mut self, details: Details) -> Self {
        self.details = details;
        self
    }

    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub(crate) fn actor_bytes(&self) -> &[u8] {
        &self.actor_bytes
    }

    pub(crate) fn call_seq(&self) -> Option<Seq> {
        self.call
    }

    pub(crate) fn action_id(&self) -> &ActionId {
        &self.action
    }

    pub(crate) fn resource_ref(&self) -> Option<&ResourceRef> {
        self.resource.as_ref()
    }

    pub(crate) fn payload(&self) -> &Details {
        &self.details
    }
}
