use serde_json::Value;
use synt_actions::Risk;
use synt_journal::ActionId;

use crate::ExtensionError;
use crate::bindings::synt::extension::types::{self, Action};

const LONGEST_TITLE: usize = 200;
const LARGEST_SCHEMA: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionSpec {
    pub(crate) id: ActionId,
    pub(crate) title: String,
    pub(crate) risk: Risk,
    pub(crate) takes_target: bool,
    pub(crate) params_schema: Value,
}

impl ActionSpec {
    pub(crate) fn from_described(extension: &str, action: &Action) -> Result<Self, ExtensionError> {
        let id = action_id(extension, &action.id)?;
        let title = title(action)?;
        let params_schema = schema(action)?;

        Ok(Self {
            id,
            title,
            risk: risk(action.risk),
            takes_target: action.takes_target,
            params_schema,
        })
    }

    pub(crate) fn at_least(self, risk: Risk) -> Self {
        Self {
            risk: self.risk.max(risk),
            ..self
        }
    }
}

fn action_id(extension: &str, id: &str) -> Result<ActionId, ExtensionError> {
    let name = id
        .strip_prefix(extension)
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or_default();

    let plain = name.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || "_-".contains(character)
    });

    if name.is_empty() || !plain {
        return Err(refused(format!(
            "action {id:?} must be named {extension}.<name>, with a lowercase name"
        )));
    }

    ActionId::new(id).map_err(|error| refused(format!("action {id:?}: {error}")))
}

fn title(action: &Action) -> Result<String, ExtensionError> {
    let title = action.title.trim();

    if title.is_empty() || title.len() > LONGEST_TITLE || title.contains(char::is_control) {
        return Err(refused(format!(
            "action {} needs a one-line title of at most {LONGEST_TITLE} bytes",
            action.id
        )));
    }

    Ok(title.to_owned())
}

fn schema(action: &Action) -> Result<Value, ExtensionError> {
    if action.params_schema.len() > LARGEST_SCHEMA {
        return Err(refused(format!(
            "the params schema of {} is over {LARGEST_SCHEMA} bytes",
            action.id
        )));
    }

    let schema: Value = serde_json::from_str(&action.params_schema).map_err(|error| {
        refused(format!(
            "the params schema of {} is not JSON: {error}",
            action.id
        ))
    })?;

    if !schema.is_object() {
        return Err(refused(format!(
            "the params schema of {} must be a JSON object",
            action.id
        )));
    }

    Ok(schema)
}

fn risk(risk: types::Risk) -> Risk {
    match risk {
        types::Risk::Read => Risk::Read,
        types::Risk::Write => Risk::Write,
        types::Risk::Privileged => Risk::Privileged,
    }
}

fn refused(reason: String) -> ExtensionError {
    ExtensionError::Component(reason)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_actions::Risk;

    use super::ActionSpec;
    use crate::bindings::synt::extension::types::{self, Action};

    fn action(id: &str, schema: &str) -> Action {
        Action {
            id: id.to_owned(),
            title: "Greet a host".to_owned(),
            risk: types::Risk::Privileged,
            takes_target: true,
            params_schema: schema.to_owned(),
        }
    }

    #[test]
    fn accepts_actions_named_after_the_extension() {
        let spec = ActionSpec::from_described("hello", &action("hello.greet", "{}")).unwrap();

        assert_eq!(spec.id.as_str(), "hello.greet");
        assert_eq!(spec.risk, Risk::Privileged);
        assert!(spec.takes_target);
        assert_eq!(spec.params_schema, json!({}));
    }

    #[test]
    fn refuses_actions_outside_its_namespace() {
        for id in [
            "ssh.exec",
            "hello",
            "hello.",
            "helloworld.greet",
            "hello.Greet",
            "hello.a.b",
        ] {
            assert!(
                ActionSpec::from_described("hello", &action(id, "{}")).is_err(),
                "{id}"
            );
        }
    }

    #[test]
    fn refuses_schemas_that_are_not_objects() {
        for schema in ["", "[]", "not json"] {
            assert!(
                ActionSpec::from_described("hello", &action("hello.greet", schema)).is_err(),
                "{schema}"
            );
        }
    }
}
