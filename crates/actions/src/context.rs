use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use synt_journal::{Actor, ResourceRef, Seq};

use crate::Risk;

#[derive(Clone)]
pub struct Context {
    actor: Actor,
    request: Seq,
    target: Option<ResourceRef>,
    risk: Risk,
    connection: Option<Arc<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn new(actor: Actor, request: Seq, target: Option<ResourceRef>) -> Self {
        Self {
            actor,
            request,
            target,
            risk: Risk::Read,
            connection: None,
        }
    }

    #[must_use]
    pub fn with_risk(mut self, risk: Risk) -> Self {
        self.risk = risk;
        self
    }

    #[must_use]
    pub fn with_connection(mut self, connection: Arc<dyn Any + Send + Sync>) -> Self {
        self.connection = Some(connection);
        self
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

    pub fn risk(&self) -> Risk {
        self.risk
    }

    pub fn connection<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let connection = Arc::clone(self.connection.as_ref()?);

        connection.downcast::<T>().ok()
    }
}

impl Debug for Context {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Context")
            .field("actor", &self.actor)
            .field("request", &self.request)
            .field("target", &self.target)
            .field("risk", &self.risk)
            .field("connection", &self.connection.is_some())
            .finish()
    }
}
