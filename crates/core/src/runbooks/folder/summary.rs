use serde::Serialize;

use crate::runbooks::RunbookName;
use crate::runbooks::document::RunbookDocument;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunbookSummary {
    name: RunbookName,
    title: String,
    description: String,
    params: usize,
    steps: usize,
    rollback: usize,
}

impl RunbookSummary {
    pub(crate) fn of(name: RunbookName, document: &RunbookDocument) -> Self {
        Self {
            name,
            title: document.name().to_string(),
            description: document.description().to_owned(),
            params: document.params().len(),
            steps: document.steps().len(),
            rollback: document.rollback().len(),
        }
    }
}
