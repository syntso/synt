use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{ExecLimits, ExecOutput, SshError, Target};
use tokio::sync::Semaphore;
use tokio::task;

use super::privilege::{escalates, is_root};

const LONGEST_TIMEOUT: Duration = Duration::from_hours(24);
const CONCURRENT_RUNS: usize = 8;

#[derive(Debug, Clone)]
pub struct SshExec {
    runs: Arc<Semaphore>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ExecParams {
    pub command: String,

    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecResult {
    pub exit: Option<u32>,
    pub signal: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration_ms: u64,
}

impl SshExec {
    pub fn new() -> Self {
        Self {
            runs: Arc::new(Semaphore::new(CONCURRENT_RUNS)),
        }
    }
}

impl Default for SshExec {
    fn default() -> Self {
        Self::new()
    }
}

impl Action for SshExec {
    const ID: &'static str = "ssh.exec";
    const RISK: Risk = Risk::Write;

    type Params = ExecParams;
    type Output = ExecResult;

    fn risk(&self, target: Option<&ResourceRef>, params: &ExecParams) -> Risk {
        if escalates(&params.command) || logs_in_as_root(target) {
            return Risk::Privileged;
        }

        Risk::Write
    }

    fn summary(&self, output: &ExecResult) -> Value {
        json!({
            "exit": output.exit,
            "signal": output.signal,
            "duration_ms": output.duration_ms,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "truncated": output.truncated,
        })
    }

    async fn run(&self, context: &Context, params: ExecParams) -> Result<ExecResult, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;

        if params.command.trim().is_empty() {
            return Err(ActionError::InvalidParams(
                "the command is empty".to_owned(),
            ));
        }

        let limits = limits(params.timeout_seconds)?;
        let target = resolve(host.as_str()).await?;

        if is_root(target.user()) && self.risk(Some(host), &params) != Risk::Privileged {
            return Err(hidden_root(host));
        }

        let _slot = self
            .runs
            .acquire()
            .await
            .map_err(|_| ActionError::failed("ssh.exec is shutting down"))?;

        let output = synt_ssh::exec(&target, &params.command, limits)
            .await
            .map_err(|error| failure(&error))?;

        Ok(ExecResult::from(output))
    }
}

impl From<ExecOutput> for ExecResult {
    fn from(output: ExecOutput) -> Self {
        Self {
            exit: output.exit,
            signal: output.signal,
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
            duration_ms: output.duration_ms,
        }
    }
}

fn logs_in_as_root(target: Option<&ResourceRef>) -> bool {
    let Some(target) = target else {
        return false;
    };

    match Target::resolve(target.as_str()) {
        Ok(resolved) => is_root(resolved.user()),
        Err(_) => true,
    }
}

fn hidden_root(host: &ResourceRef) -> ActionError {
    ActionError::Failed(format!(
        "the ssh config for {host} changed to log in as root after the call was approved; \
         run it again so it is asked as privileged"
    ))
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

async fn resolve(spec: &str) -> Result<Target, ActionError> {
    let spec = spec.to_owned();

    let resolving = task::spawn_blocking(move || Target::resolve(&spec));

    let resolved = resolving
        .await
        .map_err(|_| ActionError::failed("reading the ssh config stopped unexpectedly"))?;

    resolved.map_err(|error| failure(&error))
}

fn failure(error: &SshError) -> ActionError {
    let reason = match error {
        SshError::HostKeyUnknown { host, fingerprint } => format!(
            "the host key of {host} is not in known_hosts (fingerprint {fingerprint}); \
             check the fingerprint and add the key to ~/.ssh/known_hosts before trying again"
        ),
        SshError::HostKeyChanged { host, fingerprint } => format!(
            "the host key of {host} does not match known_hosts (server sent {fingerprint}); \
             someone may be intercepting the connection, so it was refused"
        ),
        SshError::HostKeyOtherType { host, fingerprint } => format!(
            "{host} presented a key of a type not pinned in known_hosts (fingerprint \
             {fingerprint}), while a key of another type is pinned; do not replace the pinned \
             key without checking the new one out of band"
        ),
        SshError::Timeout(limit) => {
            format!("the command was not started within {} s", limit.as_secs())
        }
        SshError::CommandTimeout { .. } => error.to_string(),
        other => format!("ssh: {other}"),
    };

    ActionError::Failed(reason)
}
