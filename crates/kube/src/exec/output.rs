use std::time::Duration;

use serde::Serialize;

use super::{Collected, ExitStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecOutput {
    pub exit: Option<i32>,
    pub failure: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration_ms: u64,
}

impl ExecOutput {
    pub(crate) fn new(
        status: ExitStatus,
        stdout: &Collected,
        stderr: &Collected,
        duration: Duration,
    ) -> Self {
        let (exit, failure) = match status {
            ExitStatus::Exited(code) => (Some(code), None),
            ExitStatus::Failed(reason) => (None, Some(reason)),
        };

        Self {
            exit,
            failure,
            stdout: stdout.text(),
            stderr: stderr.text(),
            truncated: stdout.truncated() || stderr.truncated(),
            duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        }
    }
}
