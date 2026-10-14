use synt_proto::{ActionId, Actor, ResourceRef};

use super::{Details, EventKind};
use crate::codec::Encoder;
use crate::seq::Seq;

#[derive(Debug, Clone, PartialEq)]
pub struct NewEvent {
    pub(crate) actor: Actor,
    pub(crate) actor_bytes: Vec<u8>,
    pub(crate) kind: EventKind,
    pub(crate) call: Option<Seq>,
    pub(crate) action: ActionId,
    pub(crate) resource: Option<ResourceRef>,
    pub(crate) details: Details,
}

impl NewEvent {
    pub fn new(actor: Actor, kind: EventKind, action: ActionId) -> Self {
        let actor_bytes = Encoder::actor_bytes(&actor);

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
}
