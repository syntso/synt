use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{ExecOutcome, ExecRequest};
use synt_proto::ExecLimits;

use super::Kube;
use crate::CoreError;
use crate::builtin::ssh::escalates;

#[derive(Debug, Clone)]
pub(crate) struct K8sExec {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ExecParams {
    #[serde(flatten)]
    pub request: ExecRequest,

    #[serde(default, rename = "timeout_seconds")]
    pub limits: ExecLimits,
}

impl K8sExec {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn exec(&self, context: &Context, params: &ExecParams) -> Result<ExecOutcome, CoreError> {
        let cluster = self.kube.cluster(context).await?;
        let stop = context.stop().cancelled();

        Ok(params.request.run(&cluster, params.limits, stop).await?)
    }
}

impl Action for K8sExec {
    const ID: &'static str = "k8s.exec";
    const TITLE: &'static str = "Run a command in a container of a pod";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ExecParams;
    type Output = ExecOutcome;

    fn check(&self, target: Option<&ResourceRef>, _params: &ExecParams) -> Result<(), ActionError> {
        self.kube.reach(target)?;

        Ok(())
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &ExecParams) -> Risk {
        if escalates(&params.request.command.line()) {
            return Risk::Privileged;
        }

        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ExecParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, outcome: &ExecOutcome) -> Value {
        let output = &outcome.output;

        json!({
            "exit": outcome.exit,
            "failure": outcome.failure,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "truncated": output.truncated,
            "duration_ms": output.duration_ms,
        })
    }

    async fn run(&self, context: &Context, params: ExecParams) -> Result<ExecOutcome, ActionError> {
        Ok(self.exec(context, &params).await?)
    }
}
