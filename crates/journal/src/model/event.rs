use serde::Serialize;

use super::{ActionId, Actor, EventKind, ResourceRef, Seq, Timestamp};
use crate::chain::Link;
use crate::payload::Details;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub(crate) seq: Seq,
    pub(crate) at: Timestamp,
    pub(crate) actor: Actor,
    pub(crate) kind: EventKind,
    pub(crate) call: Option<Seq>,
    pub(crate) action: ActionId,
    pub(crate) resource: Option<ResourceRef>,
    pub(crate) details: Details,
    pub(crate) link: Link,
}

impl Event {
    pub fn seq(&self) -> Seq {
        self.seq
    }

    pub fn at(&self) -> Timestamp {
        self.at
    }

    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    pub fn kind(&self) -> EventKind {
        self.kind
    }

    pub fn call(&self) -> Option<Seq> {
        self.call
    }

    pub fn action(&self) -> &ActionId {
        &self.action
    }

    pub fn resource(&self) -> Option<&ResourceRef> {
        self.resource.as_ref()
    }

    pub fn details(&self) -> &Details {
        &self.details
    }

    pub fn link(&self) -> Link {
        self.link
    }
}
