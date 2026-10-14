use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use crate::PlatformError;

#[derive(Debug, Clone)]
pub(super) struct Tool {
    path: PathBuf,
}

impl Tool {
    pub(super) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(super) fn succeeds<I, S>(&self, arguments: I) -> Result<bool, PlatformError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.run(arguments)?;

        Ok(output.status.success())
    }

    pub(super) fn require<I, S>(&self, arguments: I) -> Result<String, PlatformError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.run(arguments)?;
        let said = String::from_utf8_lossy(&output.stdout).trim().to_owned();

        if output.status.success() {
            return Ok(said);
        }

        let complaint = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let reason = if complaint.is_empty() {
            said
        } else {
            complaint
        };

        Err(PlatformError::Tool {
            tool: self.path.clone(),
            reason,
        })
    }

    fn run<I, S>(&self, arguments: I) -> Result<Output, PlatformError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.path)
            .args(arguments)
            .stdin(Stdio::null())
            .output()?;

        Ok(output)
    }
}
