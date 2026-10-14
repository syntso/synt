use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{ExecLimits, ExecOutput, ExecRequest};

use super::Kube;
use super::failure::failed;
use super::names::{check_namespace, check_object_name};
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

impl ExecParams {
    fn limits(&self) -> Result<ExecLimits, ActionError> {
        let defaults = ExecLimits::default();

        let Some(seconds) = self.timeout_seconds else {
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

    fn request(self) -> ExecRequest {
        ExecRequest {
            pod: self.pod,
            namespace: self.namespace,
            container: self.container,
            command: self.command,
        }
    }
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

    fn check(&self, target: Option<&ResourceRef>, params: &ExecParams) -> Result<(), ActionError> {
        self.kube.reach(target)?;

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

        params.limits().map(|_| ())
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
        let limits = params.limits()?;
        let cluster = self.kube.cluster(context).await?;
        let request = params.request();

        cluster
            .exec(&request, limits, context.stop().cancelled())
            .await
            .map_err(failed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synt_kube::ExecLimits;

    use super::ExecParams;

    fn params(timeout_seconds: Option<u64>) -> ExecParams {
        ExecParams {
            pod: "api".to_owned(),
            namespace: None,
            container: None,
            command: vec!["true".to_owned()],
            timeout_seconds,
        }
    }

    #[test]
    fn no_timeout_keeps_the_default_limits() {
        assert_eq!(params(None).limits().unwrap(), ExecLimits::default());
    }

    #[test]
    fn a_timeout_replaces_only_the_time_limit() {
        let limits = params(Some(30)).limits().unwrap();

        assert_eq!(limits.timeout, Duration::from_secs(30));
        assert_eq!(limits.output_bytes, ExecLimits::default().output_bytes);
    }

    #[test]
    fn a_timeout_of_zero_or_over_a_day_is_refused() {
        assert!(params(Some(0)).limits().is_err());
        assert!(params(Some(24 * 60 * 60 + 1)).limits().is_err());
    }
}
