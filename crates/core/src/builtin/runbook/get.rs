use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::RunbookParams;
use crate::blocking::blocking;
use crate::runbooks::{RunbookEntry, Runbooks};

#[derive(Debug, Clone)]
pub struct RunbookGet {
    runbooks: Runbooks,
}

impl RunbookGet {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookGet {
    const ID: &'static str = "runbook.get";
    const TITLE: &'static str = "Show one runbook of a project";
    const RISK: Risk = Risk::Read;

    type Params = RunbookParams;
    type Output = RunbookEntry;

    fn summary(&self, output: &RunbookEntry) -> Value {
        json!({ "name": output.name })
    }

    async fn run(
        &self,
        _context: &Context,
        params: RunbookParams,
    ) -> Result<RunbookEntry, ActionError> {
        let runbooks = self.runbooks.clone();

        blocking(move || {
            let (_, runbook) = runbooks.store().get(&params.project, &params.name)?;

            Ok(RunbookEntry {
                name: params.name,
                runbook,
            })
        })
        .await
    }
}
