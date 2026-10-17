use std::sync::Arc;

use serde_json::Value;
use synt_journal::{ActionId, ResourceRef};

use crate::dynamic::Dynamic;
use crate::erased::Erased;
use crate::typed::Typed;
use crate::{Action, ActionDescription, ActionError, DynamicAction, Prepared, Risk};

#[derive(Clone)]
pub struct RegisteredAction {
    id: ActionId,
    action: Arc<dyn Erased>,
}

impl RegisteredAction {
    pub(crate) fn new<A: Action>(id: ActionId, action: A) -> Self {
        Self {
            id,
            action: Arc::new(Typed::new(action)),
        }
    }

    pub fn dynamic<D: DynamicAction>(id: ActionId, action: D) -> Self {
        Self {
            id,
            action: Arc::new(Dynamic::new(action)),
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

    pub fn describe(&self) -> ActionDescription {
        ActionDescription {
            id: self.id.clone(),
            title: self.action.title().to_owned(),
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
