use std::future::Future;

use schemars::JsonSchema;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use synt_journal::ResourceRef;

use crate::{ActionError, Context, Risk};

pub trait Action: Send + Sync + 'static {
    const ID: &'static str;
    const TITLE: &'static str;
    const RISK: Risk;
    const PERSON_ONLY: bool = false;
    const TAKES_TARGET: bool = false;
    const FINISHES_LATER: bool = false;

    type Params: DeserializeOwned + JsonSchema + Send + 'static;
    type Output: Serialize;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        _params: &Self::Params,
    ) -> Result<(), ActionError> {
        Ok(())
    }

    fn risk(&self, _target: Option<&ResourceRef>, _params: &Self::Params) -> Risk {
        Self::RISK
    }

    fn destination(&self, _target: Option<&ResourceRef>, _params: &Self::Params) -> Option<String> {
        None
    }

    fn summary(&self, _output: &Self::Output) -> Value {
        Value::Null
    }

    fn recorded_params(&self, params: &Value) -> Value {
        params.clone()
    }

    fn run(
        &self,
        context: &Context,
        params: Self::Params,
    ) -> impl Future<Output = Result<Self::Output, ActionError>> + Send;
}
