use super::DraftStep;
use super::literals::Literals;
use super::placeholders::Placeholders;
use crate::runbooks::document::Runbook;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    name: String,
    steps: Vec<DraftStep>,
}

impl Draft {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, step: DraftStep) {
        self.steps.push(step);
    }

    pub(crate) fn build(self) -> Result<Runbook, String> {
        let literals = Literals::repeated_in(&self.steps);
        let placeholders = Placeholders::naming(&literals);

        let steps = self
            .steps
            .iter()
            .map(|step| step.templated(&placeholders))
            .collect();

        let runbook = Runbook {
            name: self.name,
            description: String::new(),
            params: placeholders.specs(&literals),
            steps,
            rollback: Vec::new(),
        };

        runbook.check()?;
        Ok(runbook)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::{Draft, DraftStep};
    use crate::runbooks::document::Runbook;
    use crate::runbooks::param::{ParamKind, Values};
    use crate::runbooks::template::JsonTemplate;

    fn call(action: &str, target: Option<&str>, params: &Value) -> DraftStep {
        let params: Map<String, Value> = params.as_object().cloned().unwrap();

        DraftStep::new(action.to_owned(), target.map(str::to_owned), params)
    }

    fn step(target: Option<&str>, params: &Value) -> DraftStep {
        call("ssh.exec", target, params)
    }

    fn draft(steps: Vec<DraftStep>) -> Runbook {
        let mut draft = Draft::new("From the journal");

        for step in steps {
            draft.push(step);
        }

        draft.build().unwrap()
    }

    #[test]
    fn a_target_used_by_several_steps_becomes_a_param() {
        let runbook = draft(vec![
            step(
                Some("@lab/api-1"),
                &json!({ "command": "systemctl stop app" }),
            ),
            step(
                Some("@lab/api-1"),
                &json!({ "command": "systemctl start app" }),
            ),
        ]);

        assert_eq!(runbook.params.len(), 1);
        assert_eq!(runbook.params[0].name, "host");
        assert_eq!(runbook.params[0].kind, ParamKind::Target);
        assert_eq!(runbook.params[0].default, Some(json!("@lab/api-1")));
        assert_eq!(runbook.steps[1].target.as_deref(), Some("{host}"));
    }

    #[test]
    fn values_used_once_stay_literal() {
        let runbook = draft(vec![
            step(Some("api-1"), &json!({ "command": "uptime" })),
            step(Some("api-2"), &json!({ "command": "df -h" })),
        ]);

        assert!(runbook.params.is_empty());
        assert_eq!(runbook.steps[0].target.as_deref(), Some("api-1"));
    }

    #[test]
    fn repeated_param_values_are_named_after_their_field() {
        let runbook = draft(vec![
            call("sftp.read", Some("a"), &json!({ "path": "/etc/app.conf" })),
            call("sftp.stat", Some("b"), &json!({ "path": "/etc/app.conf" })),
        ]);

        assert_eq!(runbook.params[0].name, "path");
        assert_eq!(runbook.params[0].kind, ParamKind::Word);
        assert_eq!(runbook.steps[0].params["path"], json!("{path}"));
    }

    #[test]
    fn clashing_names_get_numbers() {
        let runbook = draft(vec![
            step(Some("a"), &json!({ "command": "x" })),
            step(Some("a"), &json!({ "command": "x" })),
            step(Some("c"), &json!({ "command": "y" })),
            step(Some("d"), &json!({ "command": "y" })),
        ]);
        let names: Vec<&str> = runbook
            .params
            .iter()
            .map(|spec| spec.name.as_str())
            .collect();

        assert_eq!(names, ["host", "command", "command_2"]);
        assert_eq!(runbook.steps[3].params["command"], json!("{command_2}"));
    }

    #[test]
    fn literal_braces_survive_as_written() {
        let runbook = draft(vec![step(
            Some("a"),
            &json!({ "command": "awk '{print $1}' f" }),
        )]);

        let rendered = runbook.steps[0].params.render(&Values::default()).unwrap();

        assert_eq!(rendered["command"], json!("awk '{print $1}' f"));
        assert_eq!(runbook.steps[0].title, "awk '{{print $1}}' f");
    }
}
