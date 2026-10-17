mod capped_text;
mod limits;

use std::time::Duration;

use serde::Serialize;

pub use capped_text::CappedText;
pub use limits::ExecLimits;

use crate::Millis;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExecOutput {
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
    pub duration_ms: Millis,
}

impl ExecOutput {
    pub fn new(stdout: CappedText, stderr: CappedText, elapsed: Duration) -> Self {
        let truncated = stdout.truncated() || stderr.truncated();

        Self {
            stdout: stdout.into_text(),
            stderr: stderr.into_text(),
            truncated,
            duration_ms: Millis::from(elapsed),
        }
    }
}
