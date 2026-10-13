use std::sync::Arc;
use std::time::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{ExecLimits, ExecOutput, Pool};
use tokio::sync::Semaphore;

use super::privilege::escalates;
use super::{destination, failure, logs_in_as_root, reach_host};
use crate::projects::Projects;

const LONGEST_TIMEOUT: Duration = Duration::from_hours(24);
const CONCURRENT_RUNS: usize = 8;

#[derive(Debug, Clone)]
pub struct SshExec {
    pool: Pool,
    projects: Projects,
    runs: Arc<Semaphore>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
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
    pub kex: Option<String>,
    pub cipher: Option<String>,
}

impl SshExec {
    pub fn new(pool: Pool, projects: Projects) -> Self {
        Self {
            pool,
            projects,
            runs: Arc::new(Semaphore::new(CONCURRENT_RUNS)),
        }
    }
}

impl Action for SshExec {
    const ID: &'static str = "ssh.exec";
    const TITLE: &'static str = "Run a command on a host over SSH";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ExecParams;
    type Output = ExecResult;

    fn risk(&self, target: Option<&ResourceRef>, params: &ExecParams) -> Risk {
        if escalates(&params.command) || logs_in_as_root(&self.projects, target) {
            return Risk::Privileged;
        }

        Risk::Write
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ExecParams) -> Option<String> {
        destination(&self.projects, target)
    }

    fn summary(&self, output: &ExecResult) -> Value {
        json!({
            "exit": output.exit,
            "signal": output.signal,
            "duration_ms": output.duration_ms,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "truncated": output.truncated,
            "kex": output.kex,
            "cipher": output.cipher,
        })
    }

    async fn run(&self, context: &Context, params: ExecParams) -> Result<ExecResult, ActionError> {
        if params.command.trim().is_empty() {
            return Err(ActionError::InvalidParams(
                "the command is empty".to_owned(),
            ));
        }

        let limits = limits(params.timeout_seconds)?;
        let target = reach_host(&self.projects, context).await?;

        let _slot = self
            .runs
            .acquire()
            .await
            .map_err(|_| ActionError::failed("ssh.exec is shutting down"))?;

        let stop = context.stop().cancelled();
        let output = synt_ssh::exec(&self.pool, &target, &params.command, limits, stop)
            .await
            .map_err(|error| failure(&error))?;

        Ok(ExecResult::from(output))
    }
}

impl From<ExecOutput> for ExecResult {
    fn from(output: ExecOutput) -> Self {
        let (kex, cipher) = output
            .algorithms
            .map(|algorithms| (algorithms.kex, algorithms.cipher))
            .unzip();

        Self {
            exit: output.exit,
            signal: output.signal,
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.truncated,
            duration_ms: output.duration_ms,
            kex,
            cipher,
        }
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
