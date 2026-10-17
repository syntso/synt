use serde::Serialize;
use serde_json::{Value, json};

use crate::CoreError;
use crate::runbooks::RunbookDocument;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Drafted {
    runbook: RunbookDocument,
    toml: String,
}

impl Drafted {
    pub(crate) fn new(runbook: RunbookDocument) -> Result<Self, CoreError> {
        let toml = runbook.to_toml().map_err(CoreError::Unprintable)?;

        Ok(Self { runbook, toml })
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "steps": self.runbook.steps().len(),
            "params": self.runbook.params().len(),
        })
    }
}
