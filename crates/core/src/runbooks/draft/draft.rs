use std::collections::BTreeMap;

use serde_json::{Map, Value};

use super::DraftStep;
use super::literal::Literal;
use crate::runbooks::document::{OnFailure, Runbook, Step};
use crate::runbooks::param::{ParamKind, ParamSpec};
use crate::runbooks::template::{Template, escape_json};

const TARGET_HINT: &str = "host";

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
        let literals = self.repeated_literals();
        let names = param_names(&literals);

        let params = literals
            .iter()
            .map(|literal| spec(&names[&literal.text], literal))
            .collect();
        let steps = self
            .steps
            .iter()
            .map(|step| templated(step, &names))
            .collect();

        let runbook = Runbook {
            name: self.name,
            description: String::new(),
            params,
            steps,
            rollback: Vec::new(),
        };

        runbook.check()?;
        Ok(runbook)
    }

    fn repeated_literals(&self) -> Vec<Literal> {
        let mut literals: Vec<Literal> = Vec::new();

        for (index, step) in self.steps.iter().enumerate() {
            if let Some(target) = &step.target {
                note(&mut literals, index, target, TARGET_HINT, true);
            }

            for (key, value) in &step.params {
                if let Some(text) = value.as_str() {
                    note(&mut literals, index, text, key, false);
                }
            }
        }

        literals.retain(Literal::repeats);
        literals
    }
}

fn note(literals: &mut Vec<Literal>, step: usize, text: &str, hint: &str, is_target: bool) {
    if text.is_empty() {
        return;
    }

    if let Some(known) = literals.iter_mut().find(|literal| literal.text == text) {
        known.steps.insert(step);
        known.is_target |= is_target;
        return;
    }

    literals.push(Literal {
        text: text.to_owned(),
        hint: hint.to_owned(),
        is_target,
        steps: [step].into(),
    });
}

fn param_names(literals: &[Literal]) -> BTreeMap<String, String> {
    let mut names = BTreeMap::new();
    let mut taken: Vec<String> = Vec::new();

    for literal in literals {
        let base = identifier(&literal.hint);
        let mut name = base.clone();
        let mut counter = 2;

        while taken.contains(&name) {
            name = format!("{base}_{counter}");
            counter += 1;
        }

        taken.push(name.clone());
        names.insert(literal.text.clone(), name);
    }

    names
}

fn identifier(hint: &str) -> String {
    let cleaned: String = hint
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();

    match cleaned.chars().next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => cleaned,
        _ => format!("_{cleaned}"),
    }
}

fn spec(name: &str, literal: &Literal) -> ParamSpec {
    let as_target = literal.is_target
        && ParamKind::Target
            .accept(&Value::from(literal.text.as_str()))
            .is_ok();

    let kind = if as_target {
        ParamKind::Target
    } else {
        ParamKind::of_literal(&literal.text)
    };

    ParamSpec::literal(name.to_owned(), kind, &literal.text)
}

fn templated(step: &DraftStep, names: &BTreeMap<String, String>) -> Step {
    let lifted = |text: &str| names.get(text).map(|name| format!("{{{name}}}"));

    let target = step
        .target
        .as_deref()
        .map(|target| lifted(target).unwrap_or_else(|| Template::escape(target)));

    let params: Map<String, Value> = step
        .params
        .iter()
        .map(|(key, value)| {
            let value = match value.as_str().and_then(lifted) {
                Some(placeholder) => Value::String(placeholder),
                None => escape_json(value),
            };

            (key.clone(), value)
        })
        .collect();

    Step {
        title: Template::escape(&step.title),
        action: step.action.clone(),
        target,
        params,
        expect: None,
        retry: None,
        on_failure: OnFailure::Stop,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::{Draft, DraftStep};
    use crate::runbooks::param::{ParamKind, Values};
    use crate::runbooks::template::render_fields;

    fn step(target: Option<&str>, params: &Value) -> DraftStep {
        let params: Map<String, Value> = params.as_object().cloned().unwrap();
        let title = params
            .get("command")
            .and_then(Value::as_str)
            .unwrap_or("step")
            .to_owned();

        DraftStep {
            title,
            action: "ssh.exec".to_owned(),
            target: target.map(str::to_owned),
            params,
        }
    }

    fn draft(steps: Vec<DraftStep>) -> crate::runbooks::document::Runbook {
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
            DraftStep {
                title: "Read".to_owned(),
                action: "sftp.read".to_owned(),
                target: Some("a".to_owned()),
                params: json!({ "path": "/etc/app.conf" })
                    .as_object()
                    .cloned()
                    .unwrap(),
            },
            DraftStep {
                title: "Stat".to_owned(),
                action: "sftp.stat".to_owned(),
                target: Some("b".to_owned()),
                params: json!({ "path": "/etc/app.conf" })
                    .as_object()
                    .cloned()
                    .unwrap(),
            },
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

        let rendered = render_fields(&runbook.steps[0].params, &Values::default()).unwrap();

        assert_eq!(rendered["command"], json!("awk '{print $1}' f"));
        assert_eq!(runbook.steps[0].title, "awk '{{print $1}}' f");
    }
}
