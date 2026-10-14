use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::builtin::blocking::blocking;
use crate::runbooks::{Runbook, RunbookEntry, Runbooks};

#[derive(Debug, Clone)]
pub struct RunbookPut {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookPutParams {
    pub project: String,
    pub name: String,
    pub runbook: Runbook,
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
        params.runbook.check().map_err(ActionError::InvalidParams)?;

        self.runbooks
            .store()
            .reference(&params.project, &params.name)
            .map(|_| ())
            .map_err(ActionError::from)
    }

    fn destination(
        &self,
        _target: Option<&ResourceRef>,
        params: &RunbookPutParams,
    ) -> Option<String> {
        self.runbooks
            .store()
            .reference(&params.project, &params.name)
            .ok()
    }

    fn summary(&self, output: &RunbookEntry) -> Value {
        json!({ "name": output.name, "steps": output.runbook.steps.len() })
    }

    async fn run(
        &self,
        context: &Context,
        params: RunbookPutParams,
    ) -> Result<RunbookEntry, ActionError> {
        let admitted = context.destination().map(str::to_owned);
        let runbooks = self.runbooks.clone();

        blocking(move || {
            runbooks.store().put(
                &params.project,
                &params.name,
                params.runbook,
                admitted.as_deref(),
            )
        })
        .await
    }
}
