use std::sync::Arc;

use serde_json::Value;
use synt_journal::ResourceRef;

use crate::bound::Bound;
use crate::{Action, ActionError, Prepared, Risk, params};

pub(crate) trait Erased: Send + Sync {
    fn title(&self) -> &str;

    fn declared_risk(&self) -> Risk;

    fn person_only(&self) -> bool;

    fn takes_target(&self) -> bool;

    fn params_schema(&self) -> Value;

    fn recorded_params(&self, params: &Value) -> Value;

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError>;
}

impl<A: Action> Erased for A {
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
        params::schema::<A::Params>()
    }

    fn recorded_params(&self, params: &Value) -> Value {
        Action::recorded_params(self, params)
    }

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        let mut params = params::parse::<A::Params>(params)?;
        Action::check(self.as_ref(), target, &params)?;

        let shown = Action::inspect(self.as_ref(), target, &mut params)?;
        let risk = Action::risk(self.as_ref(), target, &params);
        let destination = Action::destination(self.as_ref(), target, &params);

        let bound = Bound::new(self, params);
        let prepared = Prepared::new(risk, destination, Box::new(bound))
            .with_finishes_later(A::FINISHES_LATER)
            .with_shown(shown);

        Ok(prepared)
    }
}
