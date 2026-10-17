mod literal;
mod placeholders;
mod step;

pub(crate) use step::DraftStep;

use synt_proto::{OneLine, RunPhase};

use super::document::{RunbookDocument, Step};
use placeholders::Placeholders;

use crate::CoreError;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Draft {
    name: OneLine,
    steps: Vec<DraftStep>,
}

impl Draft {
    pub(crate) fn new(name: OneLine) -> Self {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    pub(crate) fn push(&mut self, step: DraftStep) {
        self.steps.push(step);
    }

    pub(crate) fn build(self) -> Result<RunbookDocument, CoreError> {
        let placeholders = Placeholders::for_steps(&self.steps)?;
        let steps = self.templated_steps(&placeholders)?;

        let document = RunbookDocument::new(self.name, placeholders.into_specs(), steps);
        document.check()?;

        Ok(document)
    }

    fn templated_steps(&self, placeholders: &Placeholders) -> Result<Vec<Step>, CoreError> {
        (1..)
            .zip(&self.steps)
            .map(|(number, step)| {
                step.templated(placeholders)
                    .map_err(|error| error.in_step(RunPhase::Steps, number))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::{Draft, DraftStep};
    use synt_proto::OneLine;

    use crate::runbooks::document::RunbookDocument;
    use crate::runbooks::param::{ParamKind, Values};
    use crate::runbooks::template::JsonTemplate;

    fn call(action: &str, target: Option<&str>, params: &Value) -> DraftStep {
        let params: Map<String, Value> = params.as_object().cloned().unwrap();

        DraftStep::new(action.to_owned(), target.map(str::to_owned), params)
    }

    fn step(target: Option<&str>, params: &Value) -> DraftStep {
        call("ssh.exec", target, params)
    }

    fn draft(steps: Vec<DraftStep>) -> RunbookDocument {
        let mut draft = Draft::new(OneLine::new("From the journal").unwrap());

        for step in steps {
            draft.push(step);
        }

        draft.build().unwrap()
    }

    fn names(runbook: &RunbookDocument) -> Vec<&str> {
        runbook
            .params()
            .iter()
            .map(|spec| spec.name().as_str())
            .collect()
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
        let host = &runbook.params()[0];

        assert_eq!(names(&runbook), ["host"]);
        assert_eq!(
            serde_json::to_value(host).unwrap(),
            json!({ "name": "host", "kind": "target", "default": "@lab/api-1" })
        );
        assert_eq!(runbook.steps()[1].target(), Some("{host}"));
    }

    #[test]
    fn values_used_once_stay_literal() {
        let runbook = draft(vec![
            step(Some("api-1"), &json!({ "command": "uptime" })),
            step(Some("api-2"), &json!({ "command": "df -h" })),
        ]);

        assert!(runbook.params().is_empty());
        assert_eq!(runbook.steps()[0].target(), Some("api-1"));
    }

    #[test]
    fn repeated_param_values_are_named_after_their_field() {
        let runbook = draft(vec![
            call("sftp.read", Some("a"), &json!({ "path": "/etc/app.conf" })),
            call("sftp.stat", Some("b"), &json!({ "path": "/etc/app.conf" })),
        ]);
        let path = serde_json::to_value(&runbook.params()[0]).unwrap();

        assert_eq!(names(&runbook), ["path"]);
        assert_eq!(path["kind"], json!(ParamKind::Word));
        assert_eq!(runbook.steps()[0].params()["path"], json!("{path}"));
    }

    #[test]
    fn clashing_names_get_numbers() {
        let runbook = draft(vec![
            step(Some("a"), &json!({ "command": "x" })),
            step(Some("a"), &json!({ "command": "x" })),
            step(Some("c"), &json!({ "command": "y" })),
            step(Some("d"), &json!({ "command": "y" })),
        ]);

        assert_eq!(names(&runbook), ["host", "command", "command_2"]);
        assert_eq!(runbook.steps()[3].params()["command"], json!("{command_2}"));
    }

    #[test]
    fn literal_braces_survive_as_written() {
        let runbook = draft(vec![step(
            Some("a"),
            &json!({ "command": "awk '{print $1}' f" }),
        )]);

        let rendered = runbook.steps()[0]
            .params()
            .render(&Values::default())
            .unwrap();

        assert_eq!(rendered["command"], json!("awk '{print $1}' f"));
        assert_eq!(runbook.steps()[0].title().as_str(), "awk '{{print $1}}' f");
    }

    #[test]
    fn a_runbook_call_cannot_become_a_step() {
        let mut draft = Draft::new(OneLine::new("Nested").unwrap());
        draft.push(call("runbook.run", None, &json!({})));

        let error = draft.build().unwrap_err();

        assert_eq!(
            error.to_string(),
            "step 1: runbook.run cannot be a step of a runbook"
        );
    }
}
