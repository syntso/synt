use serde::Serialize;

use synt_proto::{ActionId, Actor, ResourceRef};

use super::{EventKind, NewEvent};
use crate::seq::Seq;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Appended {
    pub seq: Seq,
    pub actor: Actor,
    pub kind: EventKind,
    pub call: Option<Seq>,
    pub action: ActionId,
    pub resource: Option<ResourceRef>,
}

impl Appended {
    pub(crate) fn new(seq: Seq, event: &NewEvent) -> Self {
        Self {
            seq,
            actor: event.actor.clone(),
            kind: event.kind,
            call: event.call,
            action: event.action.clone(),
            resource: event.resource.clone(),
        }
    }
}
