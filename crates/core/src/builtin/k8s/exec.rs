use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{ExecLimits, ExecOutput, ExecRequest};

use super::{K8sActions, K8sError};
use crate::builtin::ssh::escalates;

#[derive(Debug, Clone)]
pub(crate) struct K8sExec {
    kube: K8sActions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ExecParams {
    #[serde(flatten)]
    pub request: ExecRequest,

    #[serde(default, rename = "timeout_seconds")]
    pub limits: ExecLimits,
}

impl K8sExec {
    pub(super) fn new(kube: K8sActions) -> Self {
        Self { kube }
    }

    async fn exec(&self, context: &Context, params: &ExecParams) -> Result<ExecOutput, K8sError> {
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
    type Output = ExecOutput;

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

    fn summary(&self, output: &ExecOutput) -> Value {
        json!({
            "exit": output.exit,
            "failure": output.failure,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "truncated": output.truncated,
            "duration_ms": output.duration_ms,
        })
    }

    async fn run(&self, context: &Context, params: ExecParams) -> Result<ExecOutput, ActionError> {
        Ok(self.exec(context, &params).await?)
    }
}
