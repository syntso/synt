use serde::Serialize;
use serde_json::{Value, json};
use synt_journal::{ActionId, Actor, Details, EventKind, NewEvent, ResourceRef};
use synt_proto::{ErrorCode, RpcError};

use super::Rejection;
use crate::access::{Decision, Scope};

pub(crate) struct Call<'a> {
    pub(crate) actor: &'a Actor,
    pub(crate) action: &'a ActionId,
    pub(crate) target: Option<&'a ResourceRef>,
}

impl Call<'_> {
    pub(crate) fn request(&self, details: Details) -> NewEvent {
        self.event(EventKind::Requested).details(details)
    }

    pub(crate) fn step(&self, kind: EventKind, details: &impl Serialize) -> NewEvent {
        let details = Details::from_serialize(details).unwrap_or_default();

        self.event(kind).details(details)
    }

    pub(crate) fn admission(&self, decision: &Decision) -> Vec<NewEvent> {
        let started = self.step(EventKind::Started, &Value::Null);

        let details = match decision {
            Decision::Approved(scope) => json!({ "scope": scope, "by": "person" }),
            Decision::Granted(grant) => {
                json!({ "scope": Scope::Always, "grant": grant, "by": "person" })
            }
            Decision::Allow | Decision::Deny { .. } => return vec![started],
        };

        let approved = self.step(EventKind::Approved, &details);

        vec![approved, started]
    }

    pub(crate) fn denied(&self, reason: String) -> Rejection {
        let details = json!({ "reason": &reason });

        Rejection {
            step: self.step(EventKind::Denied, &details),
            error: RpcError::new(ErrorCode::DENIED, reason),
        }
    }

    pub(crate) fn failed(&self, error: RpcError) -> Rejection {
        let details = json!({ "error": &error.message });

        Rejection {
            step: self.step(EventKind::Failed, &details),
            error,
        }
    }

    fn event(&self, kind: EventKind) -> NewEvent {
        let event = NewEvent::new(self.actor.clone(), kind, self.action.clone());

        match self.target {
            Some(target) => event.resource(target.clone()),
            None => event,
        }
    }
}
