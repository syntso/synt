use serde::Serialize;

use super::IntegrationShell;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellSnippet {
    pub shell: IntegrationShell,
    pub startup_file: &'static str,
    pub snippet: &'static str,
}

impl From<IntegrationShell> for ShellSnippet {
    fn from(shell: IntegrationShell) -> Self {
        Self {
            shell,
            startup_file: shell.startup_file(),
            snippet: shell.snippet(),
        }
    }
}
