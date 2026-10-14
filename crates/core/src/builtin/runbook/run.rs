use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::runbooks::{Plan, RunReport, Runbooks};

#[derive(Debug, Clone)]
pub struct RunbookRun {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookRunParams {
    pub project: String,
    pub name: String,

    #[serde(default)]
    pub params: Map<String, Value>,

    #[serde(default)]
    pub approve_steps: bool,

    #[serde(skip)]
    plan: Option<Plan>,
}

impl RunbookRun {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookRun {
    const ID: &'static str = "runbook.run";
    const TITLE: &'static str = "Run a runbook step by step";
    const RISK: Risk = Risk::Read;

    type Params = RunbookRunParams;
    type Output = RunReport;

    fn inspect(
        &self,
        _target: Option<&ResourceRef>,
        params: &mut RunbookRunParams,
    ) -> Result<Option<Value>, ActionError> {
        let plan = self
            .runbooks
            .plan(&params.project, &params.name, &params.params)?;
        let shown = plan.shown(params.approve_steps);

        params.plan = Some(plan);

        Ok(Some(shown))
    }

    fn risk(&self, _target: Option<&ResourceRef>, params: &RunbookRunParams) -> Risk {
        if params.approve_steps {
            return Risk::Write;
        }

        Risk::Read
    }

    fn destination(
        &self,
        _target: Option<&ResourceRef>,
        params: &RunbookRunParams,
    ) -> Option<String> {
        params.plan.as_ref().map(|plan| plan.runbook.clone())
    }

    fn summary(&self, output: &RunReport) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        context: &Context,
        params: RunbookRunParams,
    ) -> Result<RunReport, ActionError> {
        let plan = params
            .plan
            .ok_or_else(|| ActionError::failed("the runbook was not planned before it ran"))?;

        self.runbooks
            .run(context, &plan, params.approve_steps)
            .await
    }
}
