use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::blocking::blocking;
use crate::runbooks::{RunbookDocument, RunbookEntry, RunbookName, Runbooks};

#[derive(Debug, Clone)]
pub struct RunbookPut {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookPutParams {
    pub project: String,
    pub name: RunbookName,
    pub runbook: RunbookDocument,
}

impl RunbookPut {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookPut {
    const ID: &'static str = "runbook.put";
    const TITLE: &'static str = "Add or replace a runbook in a project";
    const RISK: Risk = Risk::Write;

    type Params = RunbookPutParams;
    type Output = RunbookEntry;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &RunbookPutParams,
    ) -> Result<(), ActionError> {
        params.runbook.check()?;
        self.runbooks.folder(&params.project)?;

        Ok(())
    }

    fn destination(
        &self,
        _target: Option<&ResourceRef>,
        params: &RunbookPutParams,
    ) -> Option<String> {
        let folder = self.runbooks.folder(&params.project).ok()?;

        Some(folder.reference(&params.name))
    }

    fn summary(&self, output: &RunbookEntry) -> Value {
        json!({ "name": output.name(), "steps": output.runbook().steps().len() })
    }

    async fn run(
        &self,
        context: &Context,
        params: RunbookPutParams,
    ) -> Result<RunbookEntry, ActionError> {
        let admitted = context.destination().map(str::to_owned);
        let runbooks = self.runbooks.clone();

        blocking(move || {
            let folder = runbooks.folder(&params.project)?;

            Ok(folder.put(params.name, params.runbook, admitted.as_deref())?)
        })
        .await
    }
}
