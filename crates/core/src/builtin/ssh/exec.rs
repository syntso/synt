use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_proto::{ExecLimits, ExecOutput};
use synt_ssh::{ExecOutcome, Pool};
use tokio::sync::Semaphore;

use super::failure;
use super::privilege::escalates;
use crate::projects::Projects;

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

    #[serde(default, rename = "timeout_seconds")]
    pub limits: ExecLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecResult {
    pub exit: Option<u32>,
    pub signal: Option<String>,

    #[serde(flatten)]
    pub output: ExecOutput,

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
        if escalates(&params.command) {
            return Risk::Privileged;
        }

        self.projects.login_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ExecParams) -> Option<String> {
        self.projects.host_destination(target)
    }

    fn summary(&self, result: &ExecResult) -> Value {
        let output = &result.output;

        json!({
            "exit": result.exit,
            "signal": result.signal,
            "duration_ms": output.duration_ms,
            "stdout_bytes": output.stdout.len(),
            "stderr_bytes": output.stderr.len(),
            "truncated": output.truncated,
            "kex": result.kex,
            "cipher": result.cipher,
        })
    }

    async fn run(&self, context: &Context, params: ExecParams) -> Result<ExecResult, ActionError> {
        if params.command.trim().is_empty() {
            return Err(ActionError::InvalidParams(
                "the command is empty".to_owned(),
            ));
        }

        let target = self.projects.reach_host(context).await?;

        let _slot = self
            .runs
            .acquire()
            .await
            .map_err(|_| ActionError::failed("ssh.exec is shutting down"))?;

        let stop = context.stop().cancelled();
        let output = self
            .pool
            .exec(&target, &params.command, params.limits, stop)
            .await
            .map_err(failure)?;

        Ok(ExecResult::from(output))
    }
}

impl From<ExecOutcome> for ExecResult {
    fn from(outcome: ExecOutcome) -> Self {
        let (kex, cipher) = outcome
            .algorithms
            .map(|algorithms| (algorithms.kex, algorithms.cipher))
            .unzip();

        Self {
            exit: outcome.exit,
            signal: outcome.signal,
            output: outcome.output,
            kex,
            cipher,
        }
    }
}
