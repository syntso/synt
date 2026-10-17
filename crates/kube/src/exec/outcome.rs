use std::time::Duration;

use serde::Serialize;
use synt_proto::ExecOutput;

use super::{Captured, ExitStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecOutcome {
    pub exit: Option<i32>,
    pub failure: Option<String>,

    #[serde(flatten)]
    pub output: ExecOutput,
}

impl ExecOutcome {
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
            output: ExecOutput::new(stdout, stderr, duration),
        }
    }
}
