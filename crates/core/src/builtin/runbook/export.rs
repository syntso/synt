use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::RunbookParams;
use crate::blocking::blocking;
use crate::runbooks::{Runbooks, ShellScript};

#[derive(Debug, Clone)]
pub struct RunbookExport {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExportedRunbook {
    pub runbook: String,
    pub script: String,
}

impl RunbookExport {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookExport {
    const ID: &'static str = "runbook.export";
    const TITLE: &'static str = "Write a runbook out as a shell script";
    const RISK: Risk = Risk::Read;

    type Params = RunbookParams;
    type Output = ExportedRunbook;

    fn summary(&self, output: &ExportedRunbook) -> Value {
        json!({ "runbook": output.runbook, "script_bytes": output.script.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: RunbookParams,
    ) -> Result<ExportedRunbook, ActionError> {
        let runbooks = self.runbooks.clone();

        blocking(move || {
            let (reference, runbook) = runbooks.store().get(&params.project, &params.name)?;
            let script = ShellScript::new(&reference, &runbook).render();

            Ok(ExportedRunbook {
                runbook: reference,
                script,
            })
        })
        .await
    }
}
