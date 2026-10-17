use std::time::Duration;

use serde::Serialize;

use super::{Captured, ExitStatus};

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
    pub(super) fn new(captured: Captured, duration: Duration) -> Self {
        let Captured {
            status,
            stdout,
            stderr,
        } = captured;

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
