use serde::Serialize;
use serde_json::{Value, json};

use super::{RunState, StepReport};
use crate::runbooks::plan::Plan;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunReport {
    pub run: u64,
    pub runbook: String,
    pub name: String,
    pub state: RunState,
    pub steps: Vec<StepReport>,
    pub rollback: Vec<StepReport>,
}

impl RunReport {
    pub(crate) fn new(run: u64, plan: &Plan) -> Self {
        Self {
            run,
            runbook: plan.runbook.clone(),
            name: plan.name.clone(),
            state: RunState::Completed,
            steps: plan.steps.iter().map(StepReport::skipped).collect(),
            rollback: Vec::new(),
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
