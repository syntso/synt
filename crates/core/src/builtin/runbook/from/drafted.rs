use serde::Serialize;
use serde_json::{Value, json};

use crate::runbooks::{RunbookDocument, RunbookError};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Drafted {
    runbook: RunbookDocument,
    toml: String,
}

impl Drafted {
    pub(crate) fn new(runbook: RunbookDocument) -> Result<Self, RunbookError> {
        let toml = runbook.to_toml().map_err(RunbookError::Unprintable)?;

        Ok(Self { runbook, toml })
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "steps": self.runbook.steps().len(),
            "params": self.runbook.params().len(),
        })
    }
}
