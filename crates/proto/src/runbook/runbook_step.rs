use serde::{Deserialize, Serialize};

use super::{RunPhase, StepState};
use crate::Notice;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunbookStep {
    pub run: u64,
    pub runbook: String,
    pub phase: RunPhase,
    pub step: usize,
    pub title: String,
    pub action: String,
    pub state: StepState,
    pub attempt: u32,

    #[serde(default)]
    pub request: Option<u64>,

    #[serde(default)]
    pub error: Option<String>,
}

impl Notice for RunbookStep {
    const METHOD: &'static str = "runbook.step";
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::RunbookStep;
    use crate::{Notice, RunPhase, StepState};

    #[test]
    fn a_step_travels_as_a_notification() {
        let step = RunbookStep {
            run: 40,
            runbook: "@lab/restart".to_owned(),
            phase: RunPhase::Rollback,
            step: 2,
            title: "Start the old version".to_owned(),
            action: "ssh.exec".to_owned(),
            state: StepState::Retrying,
            attempt: 3,
            request: Some(44),
            error: Some("exit 1, expected 0".to_owned()),
        };

        let notification = step.to_notification().unwrap();
        let params = serde_json::to_value(&step).unwrap();

        assert_eq!(notification.method, "runbook.step");
        assert_eq!(notification.params::<RunbookStep>().unwrap(), step);
        assert_eq!(params["phase"], json!("rollback"));
        assert_eq!(params["state"], json!("retrying"));
    }
}
