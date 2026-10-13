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
    runnable: Box<dyn Runnable>,
}

impl Prepared {
    pub(crate) fn new(risk: Risk, runnable: Box<dyn Runnable>) -> Self {
        Self { risk, runnable }
    }

    pub fn risk(&self) -> Risk {
        self.risk
    }

    pub async fn run(self, context: &Context) -> Result<Completed, ActionError> {
        self.runnable.run(context).await
    }
}
