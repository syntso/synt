use serde::Serialize;
use serde_json::{Value, json};
use synt_proto::{ActionOutcome, RpcError, RunPhase, RunbookStep, StepState};

use crate::runbooks::plan::PlannedStep;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StepReport {
    step: usize,
    title: String,
    action: String,
    target: Option<String>,
    state: StepState,
    attempts: u32,
    request: Option<u64>,
    error: Option<String>,
    value: Option<Value>,
}

impl StepReport {
    pub(crate) fn skipped(step: &PlannedStep) -> Self {
        Self {
            step: step.number(),
            title: step.title().to_owned(),
            action: step.action().as_str().to_owned(),
            target: step.target_text(),
            state: StepState::Skipped,
            attempts: 0,
            request: None,
            error: None,
            value: None,
        }
    }

    pub(crate) fn state(&self) -> StepState {
        self.state
    }

    pub(crate) fn succeeded(&self) -> bool {
        self.state == StepState::Succeeded
    }

    pub(crate) fn failed(&self) -> bool {
        self.state == StepState::Failed
    }

    pub(crate) fn begin_attempt(&mut self, attempt: u32) {
        self.attempts = attempt;
    }

    pub(crate) fn settle(&mut self, step: &PlannedStep, outcome: Result<ActionOutcome, RpcError>) {
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => return self.fail(None, None, error.message),
        };

        let request = Some(outcome.request.0);

        if let Err(unmet) = step.expect().judge(&outcome.value) {
            return self.fail(request, Some(outcome.value), unmet.to_string());
        }

        self.state = StepState::Succeeded;
        self.request = request;
        self.error = None;
        self.value = Some(outcome.value);
    }

    pub(crate) fn notice(
        &self,
        run: u64,
        runbook: &str,
        phase: RunPhase,
        state: StepState,
    ) -> RunbookStep {
        let settled = state != StepState::Started;

        RunbookStep {
            run,
            runbook: runbook.to_owned(),
            phase,
            step: self.step,
            title: self.title.clone(),
            action: self.action.clone(),
            state,
            attempt: self.attempts,
            request: self.request.filter(|_| settled),
            error: self.error.clone().filter(|_| settled),
        }
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
