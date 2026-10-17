use serde::Serialize;
use serde_json::{Value, json};

use super::{RunState, StepReport};
use crate::runbooks::plan::Plan;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunReport {
    run: u64,
    runbook: String,
    name: String,
    state: RunState,
    steps: Vec<StepReport>,
    rollback: Vec<StepReport>,
}

impl RunReport {
    pub(crate) fn new(
        run: u64,
        plan: &Plan,
        state: RunState,
        steps: Vec<StepReport>,
        rollback: Vec<StepReport>,
    ) -> Self {
        Self {
            run,
            runbook: plan.reference().to_owned(),
            name: plan.name().to_string(),
            state,
            steps,
            rollback,
        }
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "runbook": self.runbook,
            "state": self.state,
            "steps": self.steps.iter().map(StepReport::summary).collect::<Vec<_>>(),
            "rollback": self.rollback.iter().map(StepReport::summary).collect::<Vec<_>>(),
        })
    }
}
