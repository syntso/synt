use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::PlatformError;

pub(super) fn succeeds<I, S>(tool: &Path, arguments: I) -> Result<bool, PlatformError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run(tool, arguments)?;

    Ok(output.status.success())
}

pub(super) fn require<I, S>(tool: &Path, arguments: I) -> Result<String, PlatformError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run(tool, arguments)?;
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
        tool: tool.display().to_string(),
        reason,
    })
}

fn run<I, S>(tool: &Path, arguments: I) -> Result<Output, PlatformError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(tool)
        .args(arguments)
        .stdin(Stdio::null())
        .output()?;

    Ok(output)
}
