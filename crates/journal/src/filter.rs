use crate::model::{ActionId, Actor, EventKind, ResourceRef, Seq, Timestamp};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filter {
    pub(crate) actor: Option<Actor>,
    pub(crate) kind: Option<EventKind>,
    pub(crate) action: Option<ActionId>,
    pub(crate) resource: Option<ResourceRef>,
    pub(crate) call: Option<Seq>,
    pub(crate) since: Option<Timestamp>,
    pub(crate) until: Option<Timestamp>,
}

impl Filter {
    pub fn all() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn actor(mut self, actor: Actor) -> Self {
        self.actor = Some(actor);
        self
    }

    #[must_use]
    pub fn kind(mut self, kind: EventKind) -> Self {
        self.kind = Some(kind);
        self
    }

    #[must_use]
    pub fn action(mut self, action: ActionId) -> Self {
        self.action = Some(action);
        self
    }

    #[must_use]
    pub fn resource(mut self, resource: ResourceRef) -> Self {
        self.resource = Some(resource);
        self
    }

    #[must_use]
    pub fn call(mut self, request: Seq) -> Self {
        self.call = Some(request);
        self
    }

    #[must_use]
    pub fn since(mut self, at: Timestamp) -> Self {
        self.since = Some(at);
        self
    }

    #[must_use]
    pub fn until(mut self, at: Timestamp) -> Self {
        self.until = Some(at);
        self
    }
}
