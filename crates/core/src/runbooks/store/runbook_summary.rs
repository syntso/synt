use serde::Serialize;

use crate::runbooks::document::Runbook;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunbookSummary {
    pub name: String,
    pub title: String,
    pub description: String,
    pub params: usize,
    pub steps: usize,
    pub rollback: usize,
}

impl RunbookSummary {
    pub(crate) fn of(name: String, runbook: &Runbook) -> Self {
        Self {
            name,
            title: runbook.name.clone(),
            description: runbook.description.clone(),
            params: runbook.params.len(),
            steps: runbook.steps.len(),
            rollback: runbook.rollback.len(),
        }
    }
}
