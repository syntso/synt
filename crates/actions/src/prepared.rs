use serde_json::Value;

use crate::runnable::Runnable;
use crate::{ActionError, Completed, Context, Risk};

pub struct Prepared {
    risk: Risk,
    destination: Option<String>,
    grantable: bool,
    finishes_later: bool,
    shown: Option<Value>,
    runnable: Box<dyn Runnable>,
}

impl Prepared {
    pub(crate) fn new(
        risk: Risk,
        destination: Option<String>,
        runnable: Box<dyn Runnable>,
    ) -> Self {
        Self {
            risk,
            destination,
            grantable: true,
            finishes_later: false,
            shown: None,
            runnable,
        }
    }

    #[must_use]
    pub(crate) fn with_grantable(mut self, grantable: bool) -> Self {
        self.grantable = grantable;
        self
    }

    #[must_use]
    pub(crate) fn with_finishes_later(mut self, finishes_later: bool) -> Self {
        self.finishes_later = finishes_later;
        self
    }

    #[must_use]
    pub(crate) fn with_shown(mut self, shown: Option<Value>) -> Self {
        self.shown = shown;
        self
    }

    pub fn risk(&self) -> Risk {
        self.risk
    }

    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    pub fn grantable(&self) -> bool {
        self.grantable
    }

    pub fn finishes_later(&self) -> bool {
        self.finishes_later
    }

    pub fn shown(&self) -> Option<&Value> {
        self.shown.as_ref()
    }

    pub async fn run(self, context: &Context) -> Result<Completed, ActionError> {
        self.runnable.run(context).await
    }
}
