use std::sync::Arc;

use serde_json::Value;
use synt_journal::{ActionId, ResourceRef};

use crate::bound::Bound;
use crate::{Action, ActionError, Prepared, Risk};

trait Erased: Send + Sync {
    fn declared_risk(&self) -> Risk;

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError>;
}

impl<A: Action> Erased for A {
    fn declared_risk(&self) -> Risk {
        A::RISK
    }

    fn prepare(
        self: Arc<Self>,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        let params: A::Params = serde_json::from_value(params)
            .map_err(|error| ActionError::InvalidParams(error.to_string()))?;

        let risk = Action::risk(self.as_ref(), target, &params);
        let bound = Bound::new(self, params);

        Ok(Prepared::new(risk, Box::new(bound)))
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

    pub fn prepare(
        &self,
        target: Option<&ResourceRef>,
        params: Value,
    ) -> Result<Prepared, ActionError> {
        Arc::clone(&self.action).prepare(target, params)
    }
}
