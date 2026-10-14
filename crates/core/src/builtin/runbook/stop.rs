use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::runbooks::Runbooks;

#[derive(Debug, Clone)]
pub struct RunbookStop {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookStopParams {
    pub run: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoppedRun {
    pub run: u64,
    pub runbook: String,
}

impl RunbookStop {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookStop {
    const ID: &'static str = "runbook.stop";
    const TITLE: &'static str = "Stop a running runbook";
    const RISK: Risk = Risk::Write;

    type Params = RunbookStopParams;
    type Output = StoppedRun;

    fn summary(&self, output: &StoppedRun) -> Value {
        json!({ "run": output.run, "runbook": output.runbook })
    }

    async fn run(
        &self,
        _context: &Context,
        params: RunbookStopParams,
    ) -> Result<StoppedRun, ActionError> {
        let runbook = self.runbooks.stop(params.run).ok_or_else(|| {
            ActionError::failed(format!("there is no running run {}", params.run))
        })?;

        Ok(StoppedRun {
            run: params.run,
            runbook,
        })
    }
}
