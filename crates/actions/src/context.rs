use std::any::Any;
use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;

use synt_journal::{Actor, ResourceRef, Seq};
use tokio_util::sync::CancellationToken;

use crate::Risk;

#[derive(Clone)]
pub struct Context {
    actor: Actor,
    request: Seq,
    target: Option<ResourceRef>,
    risk: Risk,
    destination: Option<String>,
    connection: Option<Arc<dyn Any + Send + Sync>>,
    stop: CancellationToken,
    hold: Option<Arc<dyn Any + Send + Sync>>,
}

impl Context {
    pub fn new(actor: Actor, request: Seq, target: Option<ResourceRef>) -> Self {
        Self {
            actor,
            request,
            target,
            risk: Risk::Read,
            destination: None,
            connection: None,
            stop: CancellationToken::new(),
            hold: None,
        }
    }

    #[must_use]
    pub fn with_risk(mut self, risk: Risk) -> Self {
        self.risk = risk;
        self
    }

    #[must_use]
    pub fn with_destination(mut self, destination: Option<String>) -> Self {
        self.destination = destination;
        self
    }

    #[must_use]
    pub fn with_connection(mut self, connection: Arc<dyn Any + Send + Sync>) -> Self {
        self.connection = Some(connection);
        self
    }

    #[must_use]
    pub fn with_stop(mut self, stop: CancellationToken) -> Self {
        self.stop = stop;
        self
    }

    #[must_use]
    pub fn with_hold(mut self, hold: Arc<dyn Any + Send + Sync>) -> Self {
        self.hold = Some(hold);
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

    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    pub fn stop(&self) -> &CancellationToken {
        &self.stop
    }

    pub fn hold(&self) -> Option<Arc<dyn Any + Send + Sync>> {
        self.hold.clone()
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
            .field("destination", &self.destination)
            .field("connection", &self.connection.is_some())
            .field("stopped", &self.stop.is_cancelled())
            .field("held", &self.hold.is_some())
            .finish()
    }
}
