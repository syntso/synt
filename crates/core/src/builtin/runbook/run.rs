use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::RunbookParams;
use crate::CoreError;
use crate::runbooks::{Plan, RunReport, Runbooks};

#[derive(Debug, Clone)]
pub struct RunbookRun {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookRunParams {
    #[serde(flatten)]
    pub(crate) runbook: RunbookParams,

    #[serde(default)]
    pub(crate) params: Map<String, Value>,

    #[serde(default)]
    pub(crate) approve_steps: bool,

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
        let plan = self.runbooks.plan(
            &params.runbook.project,
            &params.runbook.name,
            &params.params,
        )?;
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
        params.plan.as_ref().map(|plan| plan.reference().to_owned())
    }

    fn grantable(&self, _target: Option<&ResourceRef>, params: &RunbookRunParams) -> bool {
        !params.approve_steps
    }

    fn summary(&self, output: &RunReport) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        context: &Context,
        params: RunbookRunParams,
    ) -> Result<RunReport, ActionError> {
        let plan = params.plan.ok_or(CoreError::NotPlanned)?;

        let report = self
            .runbooks
            .run(context, &plan, params.approve_steps)
            .await?;

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::RunbookRunParams;

    #[test]
    fn run_params_take_the_runbook_fields_and_nothing_else() {
        let parsed: RunbookRunParams = serde_json::from_value(json!({
            "project": "orders",
            "name": "deploy",
            "params": { "service": "api" },
        }))
        .unwrap();

        assert_eq!(parsed.runbook.project.as_str(), "orders");
        assert!(!parsed.approve_steps);

        let unknown = serde_json::from_value::<RunbookRunParams>(json!({
            "project": "orders",
            "name": "deploy",
            "approve": true,
        }));
        assert!(unknown.is_err());
    }
}
