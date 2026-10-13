use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use synt_journal::ActionId;

use crate::{Action, ActionError, Context, Risk};

type Running<'a> = Pin<Box<dyn Future<Output = Result<Value, ActionError>> + Send + 'a>>;

trait Erased: Send + Sync {
    fn risk(&self) -> Risk;

    fn call<'a>(&'a self, context: &'a Context, params: Value) -> Running<'a>;
}

impl<A: Action> Erased for A {
    fn risk(&self) -> Risk {
        A::RISK
    }

    fn call<'a>(&'a self, context: &'a Context, params: Value) -> Running<'a> {
        Box::pin(async move {
            let params = serde_json::from_value(params)
                .map_err(|error| ActionError::InvalidParams(error.to_string()))?;

            let output = self.run(context, params).await?;

            serde_json::to_value(output).map_err(|error| ActionError::failed(error.to_string()))
        })
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
        self.action.risk()
    }

    pub async fn call(&self, context: &Context, params: Value) -> Result<Value, ActionError> {
        self.action.call(context, params).await
    }
}
