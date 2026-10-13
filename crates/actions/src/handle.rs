use std::sync::Arc;

use serde_json::Value;
use synt_journal::{ActionId, ResourceRef};

use crate::bound::Bound;
use crate::{Action, ActionError, Description, Prepared, Risk, params};

trait Erased: Send + Sync {
    fn title(&self) -> &'static str;

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
    fn title(&self) -> &'static str {
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
        let params = params::normalized::<A::Params>(params);
        let params: A::Params = serde_json::from_value(params)
            .map_err(|error| ActionError::InvalidParams(error.to_string()))?;
        Action::check(self.as_ref(), target, &params)?;

        let risk = Action::risk(self.as_ref(), target, &params);
        let destination = Action::destination(self.as_ref(), target, &params);
        let bound = Bound::new(self, params);

        Ok(Prepared::new(
            risk,
            destination,
            A::FINISHES_LATER,
            Box::new(bound),
        ))
    }
}

#[derive(Clone)]
pub struct Handle {
    id: ActionId,
    action: Arc<dyn Erased>,
}

impl Handle {
    pub(crate) fn new<A: Action>(id: ActionId, action: A) -> Self {
        Self {
            id,
            action: Arc::new(action),
        }
    }

    pub fn id(&self) -> &ActionId {
        &self.id
    }

    pub fn risk(&self) -> Risk {
        self.action.declared_risk()
    }

    pub fn person_only(&self) -> bool {
        self.action.person_only()
    }

    pub fn recorded_params(&self, params: &Value) -> Value {
        self.action.recorded_params(params)
    }

    pub fn takes_target(&self) -> bool {
        self.action.takes_target()
    }

    pub fn describe(&self) -> Description {
        Description {
            id: self.id.clone(),
            title: self.action.title(),
            risk: self.action.declared_risk(),
            person_only: self.action.person_only(),
            takes_target: self.action.takes_target(),
            params: self.action.params_schema(),
        }
    }

    pub fn prepare(
        &self,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        Arc::clone(&self.action).prepare(target, params)
    }
}
