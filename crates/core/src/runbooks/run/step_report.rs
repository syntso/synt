use serde::Serialize;
use serde_json::{Value, json};
use synt_proto::{ActionOutcome, RpcError, StepState};

use crate::runbooks::plan::PlannedStep;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepReport {
    pub step: usize,
    pub title: String,
    pub action: String,
    pub target: Option<String>,
    pub state: StepState,
    pub attempts: u32,
    pub request: Option<u64>,
    pub error: Option<String>,
    pub value: Option<Value>,
}

impl StepReport {
    pub(crate) fn skipped(step: &PlannedStep) -> Self {
        Self {
            step: step.number,
            title: step.title.clone(),
            action: step.action.as_str().to_owned(),
            target: step.target_text(),
            state: StepState::Skipped,
            attempts: 0,
            request: None,
            error: None,
            value: None,
        }
    }

    pub(crate) fn settle(&mut self, step: &PlannedStep, outcome: Result<ActionOutcome, RpcError>) {
        let (request, value) = match outcome {
            Ok(outcome) => (Some(outcome.request.get()), outcome.value),
            Err(error) => {
                self.fail(None, None, error.message);
                return;
            }
        };

        match step.expect.judge(&value) {
            Ok(()) => {
                self.state = StepState::Succeeded;
                self.request = request;
                self.error = None;
                self.value = Some(value);
            }
            Err(reason) => self.fail(request, Some(value), reason),
        }
    }

    pub(crate) fn succeeded(&self) -> bool {
        self.state == StepState::Succeeded
    }

    pub(crate) fn failed(&self) -> bool {
        self.state == StepState::Failed
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "step": self.step,
            "action": self.action,
            "target": self.target,
            "state": self.state,
            "attempts": self.attempts,
            "request": self.request,
        })
    }

    fn fail(&mut self, request: Option<u64>, value: Option<Value>, reason: String) {
        self.state = StepState::Failed;
        self.request = request;
        self.error = Some(reason);
        self.value = value;
    }
}
