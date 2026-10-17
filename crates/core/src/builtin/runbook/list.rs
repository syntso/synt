use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::blocking::blocking;
use crate::runbooks::{RunbookListing, Runbooks};

#[derive(Debug, Clone)]
pub struct RunbookList {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookListParams {
    pub project: String,
}

impl RunbookList {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookList {
    const ID: &'static str = "runbook.list";
    const TITLE: &'static str = "List the runbooks of a project";
    const RISK: Risk = Risk::Read;

    type Params = RunbookListParams;
    type Output = RunbookListing;

    fn summary(&self, output: &RunbookListing) -> Value {
        json!({
            "project": output.project(),
            "runbooks": output.runbooks().len(),
            "problems": output.problems().len(),
        })
    }

    async fn run(
        &self,
        _context: &Context,
        params: RunbookListParams,
    ) -> Result<RunbookListing, ActionError> {
        let runbooks = self.runbooks.clone();

        blocking(move || runbooks.folder(&params.project)?.listing()).await
    }
}
