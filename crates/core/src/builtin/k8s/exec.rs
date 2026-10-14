use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{ExecLimits, ExecOutput, ExecRequest};

use super::Kube;
use super::failure::failed;
use super::object_names::{check_namespace, check_object_name};
use crate::builtin::ssh::escalates;

const LONGEST_TIMEOUT: Duration = Duration::from_hours(24);

#[derive(Debug, Clone)]
pub(crate) struct K8sExec {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ExecParams {
    pub pod: String,

    #[serde(default)]
    pub namespace: Option<String>,

    #[serde(default)]
    pub container: Option<String>,

    pub command: Vec<String>,

    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

impl K8sExec {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sExec {
    const ID: &'static str = "k8s.exec";
    const TITLE: &'static str = "Run a command in a container of a pod";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ExecParams;
    type Output = ExecOutput;

    fn check(&self, _target: Option<&ResourceRef>, params: &ExecParams) -> Result<(), ActionError> {
        check_object_name("pod", &params.pod)?;
        check_namespace(params.namespace.as_deref())?;

        if let Some(container) = &params.container {
            check_object_name("container", container)?;
        }

        if params
            .command
            .first()
            .is_none_or(|program| program.trim().is_empty())
        {
            return Err(ActionError::InvalidParams(
                "the command is empty; give the program and its arguments as a list".to_owned(),
            ));
        }

        limits(params.timeout_seconds).map(|_| ())
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &ExecParams) -> Risk {
        if escalates(&params.command.join(" ")) {
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
        let limits = limits(params.timeout_seconds)?;
        let cluster = self.kube.cluster(context).await?;

        let request = ExecRequest {
            pod: params.pod,
            namespace: params.namespace,
            container: params.container,
            command: params.command,
        };

        cluster
            .exec(&request, limits, context.stop().cancelled())
            .await
            .map_err(failed)
    }
}

fn limits(timeout_seconds: Option<u64>) -> Result<ExecLimits, ActionError> {
    let defaults = ExecLimits::default();

    let Some(seconds) = timeout_seconds else {
        return Ok(defaults);
    };

    let timeout = Duration::from_secs(seconds);

    if timeout.is_zero() || timeout > LONGEST_TIMEOUT {
        return Err(ActionError::InvalidParams(format!(
            "timeout_seconds must be between 1 and {}, got {seconds}",
            LONGEST_TIMEOUT.as_secs()
        )));
    }

    Ok(ExecLimits {
        timeout,
        ..defaults
    })
}
