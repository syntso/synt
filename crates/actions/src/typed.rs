mod action;
mod bound;

use std::sync::Arc;

use serde_json::Value;
use synt_journal::ResourceRef;

use self::bound::Bound;
use crate::erased::Erased;
use crate::params::Params;
use crate::{ActionError, Prepared, Risk};

pub use action::Action;

pub(crate) struct Typed<A> {
    action: Arc<A>,
}

impl<A: Action> Typed<A> {
    pub(crate) fn new(action: A) -> Self {
        Self {
            action: Arc::new(action),
        }
    }

    fn bind(
        &self,
        target: Option<&ResourceRef>,
        mut params: A::Params,
    ) -> Result<Prepared, ActionError> {
        let action = self.action.as_ref();

        let shown = action.inspect(target, &mut params)?;
        let risk = action.risk(target, &params);
        let destination = action.destination(target, &params);
        let grantable = action.grantable(target, &params);
        let finishes_later = action.finishes_later(&params);

        let bound = Bound::new(Arc::clone(&self.action), params);

        let prepared = Prepared::new(risk, destination, Box::new(bound))
            .with_grantable(grantable)
            .with_finishes_later(finishes_later)
            .with_shown(shown);

        Ok(prepared)
    }
}

impl<A: Action> Erased for Typed<A> {
    fn title(&self) -> &str {
        A::TITLE
    }

    fn declared_risk(&self) -> Risk {
        A::RISK
    }

    fn person_only(&self) -> bool {
        A::PERSON_ONLY
    }

    fn takes_target(&self) -> bool {
        A::TAKES_TARGET
    }

    fn params_schema(&self) -> Value {
        A::Params::schema()
    }

    fn recorded_params(&self, params: &Value) -> Value {
        self.action.recorded_params(params)
    }

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        let params = A::Params::parse(params)?;
        self.action.check(target, &params)?;

        self.bind(target, params)
    }
}
