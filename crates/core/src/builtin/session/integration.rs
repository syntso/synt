mod shell;
mod snippet;

pub(crate) use shell::IntegrationShell;
pub(crate) use snippet::ShellSnippet;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

#[derive(Debug, Clone, Copy, Default)]
pub struct SessionIntegration;

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, JsonSchema)]
pub struct IntegrationParams {
    #[serde(default)]
    pub shell: Option<IntegrationShell>,
}

impl Action for SessionIntegration {
    const ID: &'static str = "session.integration";
    const TITLE: &'static str =
        "Show the shell snippets that mark where each command starts and ends";
    const RISK: Risk = Risk::Read;

    type Params = IntegrationParams;
    type Output = Vec<ShellSnippet>;

    fn summary(&self, output: &Vec<ShellSnippet>) -> Value {
        let shells: Vec<IntegrationShell> = output.iter().map(|snippet| snippet.shell).collect();

        json!({ "shells": shells })
    }

    async fn run(
        &self,
        _context: &Context,
        params: IntegrationParams,
    ) -> Result<Vec<ShellSnippet>, ActionError> {
        let snippets = match params.shell {
            Some(shell) => vec![ShellSnippet::from(shell)],
            None => IntegrationShell::ALL.map(ShellSnippet::from).to_vec(),
        };

        Ok(snippets)
    }
}

#[cfg(test)]
mod tests {
    use super::IntegrationShell;

    #[test]
    fn every_snippet_marks_output_start_and_command_end() {
        for shell in IntegrationShell::ALL {
            let snippet = shell.snippet();

            assert!(snippet.contains("133;A"), "{shell:?}");
            assert!(snippet.contains("133;C;cmdline_url="), "{shell:?}");
            assert!(snippet.contains("133;D;"), "{shell:?}");
        }
    }
}
