use std::fmt::{self, Display, Formatter};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synt_journal::ActionId;

use crate::runbooks::RunbookError;

const RUNBOOK_ACTIONS: &str = "runbook.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct StepAction(#[schemars(with = "String")] ActionId);

impl StepAction {
    pub(crate) fn new(action: impl Into<String>) -> Result<Self, RunbookError> {
        let action = ActionId::new(action).map_err(RunbookError::BadAction)?;

        if action.as_str().starts_with(RUNBOOK_ACTIONS) {
            return Err(RunbookError::NestedRunbook { action });
        }

        Ok(Self(action))
    }

    pub(crate) fn id(&self) -> &ActionId {
        &self.0
    }

    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for StepAction {
    type Error = RunbookError;

    fn try_from(action: String) -> Result<Self, RunbookError> {
        Self::new(action)
    }
}

impl From<StepAction> for String {
    fn from(action: StepAction) -> Self {
        action.as_str().to_owned()
    }
}

impl Display for StepAction {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::StepAction;

    #[test]
    fn a_step_calls_any_action_but_a_runbook() {
        assert_eq!(StepAction::new("ssh.exec").unwrap().as_str(), "ssh.exec");

        assert_eq!(
            StepAction::new("runbook.run").unwrap_err().to_string(),
            "runbook.run cannot be a step of a runbook"
        );
        assert!(
            StepAction::new("")
                .unwrap_err()
                .to_string()
                .starts_with("action: ")
        );
    }
}
