use serde_json::{Map, Value};

use super::placeholders::Placeholders;
use crate::runbooks::RunbookError;
use crate::runbooks::document::{OneLine, Step, StepAction};
use crate::runbooks::template::{JsonTemplate, TextTemplate};

const LONGEST_TITLE: usize = 80;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DraftStep {
    title: String,
    action: String,
    target: Option<String>,
    params: Map<String, Value>,
}

impl DraftStep {
    pub(crate) fn new(action: String, target: Option<String>, params: Map<String, Value>) -> Self {
        let title = match (params.get(Step::COMMAND).and_then(Value::as_str), &target) {
            (Some(command), _) => Self::title_from(command),
            (None, Some(target)) => format!("{action} on {target}"),
            (None, None) => action.clone(),
        };

        Self {
            title,
            action,
            target,
            params,
        }
    }

    pub(crate) fn shell(host: &str, command: String) -> Self {
        let params = Map::from_iter([(Step::COMMAND.to_owned(), Value::String(command))]);

        Self::new(Step::SHELL_ACTION.to_owned(), Some(host.to_owned()), params)
    }

    pub(super) fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    pub(super) fn params(&self) -> &Map<String, Value> {
        &self.params
    }

    pub(super) fn templated(&self, placeholders: &Placeholders) -> Result<Step, RunbookError> {
        let title = OneLine::new(TextTemplate::escape(&self.title))?;
        let action = StepAction::new(self.action.as_str())?;

        let target = self.target.as_deref().map(|target| {
            placeholders
                .lift(target)
                .unwrap_or_else(|| TextTemplate::escape(target))
        });

        let params = self
            .params
            .iter()
            .map(|(key, value)| (key.clone(), Self::templated_value(value, placeholders)))
            .collect();

        Ok(Step::new(title, action, target, params))
    }

    fn templated_value(value: &Value, placeholders: &Placeholders) -> Value {
        match value.as_str().and_then(|text| placeholders.lift(text)) {
            Some(placeholder) => Value::String(placeholder),
            None => value.escaped(),
        }
    }

    fn title_from(text: &str) -> String {
        let line = text.lines().next().unwrap_or_default().trim();
        let line: String = line
            .chars()
            .filter(|character| !character.is_control())
            .collect();

        if line.chars().count() <= LONGEST_TITLE {
            return line;
        }

        let shortened: String = line.chars().take(LONGEST_TITLE - 1).collect();
        format!("{shortened}…")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, json};

    use super::DraftStep;

    #[test]
    fn a_title_is_the_first_line_kept_short() {
        assert_eq!(DraftStep::title_from("uptime\nrm x"), "uptime");
        assert_eq!(DraftStep::title_from(&"a".repeat(100)).chars().count(), 80);
        assert!(DraftStep::title_from(&"a".repeat(100)).ends_with('…'));
    }

    #[test]
    fn a_step_without_a_command_is_titled_by_its_action_and_target() {
        let read = DraftStep::new(
            "sftp.read".to_owned(),
            Some("@lab/api-1".to_owned()),
            Map::new(),
        );
        let ping = DraftStep::new("core.ping".to_owned(), None, Map::new());

        assert_eq!(read.title, "sftp.read on @lab/api-1");
        assert_eq!(ping.title, "core.ping");
    }

    #[test]
    fn a_shell_step_runs_its_command_on_the_host() {
        let step = DraftStep::shell("@lab/api-1", "uptime\nrm x".to_owned());

        assert_eq!(step.title, "uptime");
        assert_eq!(step.action, "ssh.exec");
        assert_eq!(step.target.as_deref(), Some("@lab/api-1"));
        assert_eq!(step.params["command"], json!("uptime\nrm x"));
    }
}
