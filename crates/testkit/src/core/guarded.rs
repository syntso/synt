use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_proto::ActionCall;

pub struct Guarded {
    runs: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardedParams {
    pub risk: String,

    #[serde(default)]
    pub destination: Option<String>,
}

impl Guarded {
    pub fn new(runs: Arc<AtomicUsize>) -> Self {
        Self { runs }
    }

    pub fn call(risk: &str) -> ActionCall {
        ActionCall::new(Self::ID).params(json!({ "risk": risk }))
    }

    pub fn call_to(risk: &str, destination: &str) -> ActionCall {
        ActionCall::new(Self::ID).params(json!({ "risk": risk, "destination": destination }))
    }
}

impl Action for Guarded {
    const ID: &'static str = "test.guarded";
    const TITLE: &'static str = "Run at the risk the params name";
    const RISK: Risk = Risk::Read;

    type Params = GuardedParams;
    type Output = ();

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &GuardedParams,
    ) -> Result<(), ActionError> {
        match params.risk.as_str() {
            "read" | "write" | "privileged" => Ok(()),
            other => Err(ActionError::InvalidParams(format!("no risk named {other}"))),
        }
    }

    fn risk(&self, _target: Option<&ResourceRef>, params: &GuardedParams) -> Risk {
        match params.risk.as_str() {
            "privileged" => Risk::Privileged,
            "write" => Risk::Write,
            _ => Risk::Read,
        }
    }

    fn destination(&self, _target: Option<&ResourceRef>, params: &GuardedParams) -> Option<String> {
        params.destination.clone()
    }

    async fn run(&self, _context: &Context, _params: GuardedParams) -> Result<(), ActionError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
