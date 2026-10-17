use serde::Serialize;

use crate::runbooks::RunbookName;
use crate::runbooks::document::RunbookDocument;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RunbookEntry {
    name: RunbookName,
    runbook: RunbookDocument,
}

impl RunbookEntry {
    pub(crate) fn new(name: RunbookName, runbook: RunbookDocument) -> Self {
        Self { name, runbook }
    }

    pub(crate) fn name(&self) -> &RunbookName {
        &self.name
    }

    pub(crate) fn runbook(&self) -> &RunbookDocument {
        &self.runbook
    }
}
