use std::future::Future;
use std::pin::Pin;

use crate::{ActionError, Completed, Context, Risk};

pub(crate) type Running<'a> =
    Pin<Box<dyn Future<Output = Result<Completed, ActionError>> + Send + 'a>>;

pub(crate) trait Runnable: Send {
    fn run(self: Box<Self>, context: &Context) -> Running<'_>;
}

pub struct Prepared {
    risk: Risk,
    destination: Option<String>,
    finishes_later: bool,
    runnable: Box<dyn Runnable>,
}

impl Prepared {
    pub(crate) fn new(
        risk: Risk,
        destination: Option<String>,
        finishes_later: bool,
        runnable: Box<dyn Runnable>,
    ) -> Self {
        Self {
            risk,
            destination,
            finishes_later,
            runnable,
        }
    }

    pub fn risk(&self) -> Risk {
        self.risk
    }

    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    pub fn finishes_later(&self) -> bool {
        self.finishes_later
    }

    pub async fn run(self, context: &Context) -> Result<Completed, ActionError> {
        self.runnable.run(context).await
    }
}
