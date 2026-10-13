use synt_journal::{Actor, ResourceRef, Seq};

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    actor: Actor,
    request: Seq,
    target: Option<ResourceRef>,
}

impl Context {
    pub fn new(actor: Actor, request: Seq, target: Option<ResourceRef>) -> Self {
        Self {
            actor,
            request,
            target,
        }
    }

    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    pub fn request(&self) -> Seq {
        self.request
    }

    pub fn target(&self) -> Option<&ResourceRef> {
        self.target.as_ref()
    }
}
