use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::ActionError;

use crate::runbooks::Runbook;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Drafted {
    pub runbook: Runbook,
    pub toml: String,
}

impl Drafted {
    pub(crate) fn new(runbook: Runbook) -> Result<Self, ActionError> {
        let toml = runbook.to_toml().map_err(ActionError::failed)?;

        Ok(Self { runbook, toml })
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "steps": self.runbook.steps.len(),
            "params": self.runbook.params.len(),
        })
    }
}
