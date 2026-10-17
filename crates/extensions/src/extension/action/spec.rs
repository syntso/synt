use serde_json::Value;
use synt_actions::Risk;
use synt_journal::ActionId;

use crate::ExtensionError;
use crate::interface::DescribedAction;
use crate::manifest::OneLine;

const LARGEST_SCHEMA: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActionSpec {
    id: ActionId,
    title: OneLine,
    risk: Risk,
    takes_target: bool,
    params_schema: Value,
}

impl ActionSpec {
    pub(crate) fn from_described(
        extension: &str,
        action: &DescribedAction,
    ) -> Result<Self, ExtensionError> {
        let id = Self::parse_id(extension, action.id())?;
        let title = Self::parse_title(action)?;
        let params_schema = Self::parse_schema(action)?;

        Ok(Self {
            id,
            title,
            risk: action.risk(),
            takes_target: action.takes_target(),
            params_schema,
        })
    }

    #[must_use]
    pub(crate) fn at_least(self, risk: Risk) -> Self {
        Self {
            risk: self.risk.max(risk),
            ..self
        }
    }

    pub(crate) fn id(&self) -> &ActionId {
        &self.id
    }

    pub(crate) fn title(&self) -> &str {
        self.title.as_str()
    }

    pub(crate) fn risk(&self) -> Risk {
        self.risk
    }

    pub(crate) fn takes_target(&self) -> bool {
        self.takes_target
    }

    pub(crate) fn params_schema(&self) -> &Value {
        &self.params_schema
    }

    fn parse_id(extension: &str, id: &str) -> Result<ActionId, ExtensionError> {
        let name = id
            .strip_prefix(extension)
            .and_then(|rest| rest.strip_prefix('.'))
            .unwrap_or_default();

        let plain = name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || "_-".contains(character)
        });

        if name.is_empty() || !plain {
            return Err(ExtensionError::ActionOutsideNamespace {
                action: id.to_owned(),
                extension: extension.to_owned(),
            });
        }

        ActionId::new(id).map_err(|source| ExtensionError::ActionId {
            action: id.to_owned(),
            source,
        })
    }

    fn parse_title(action: &DescribedAction) -> Result<OneLine, ExtensionError> {
        let title = action.title().trim().to_owned();

        OneLine::try_from(title).map_err(|_| ExtensionError::ActionTitle {
            action: action.id().to_owned(),
            longest: OneLine::LONGEST,
        })
    }

    fn parse_schema(action: &DescribedAction) -> Result<Value, ExtensionError> {
        let text = action.params_schema();

        if text.len() > LARGEST_SCHEMA {
            return Err(ExtensionError::SchemaTooLarge {
                action: action.id().to_owned(),
                largest: LARGEST_SCHEMA,
            });
        }

        let schema: Value =
            serde_json::from_str(text).map_err(|source| ExtensionError::SchemaNotJson {
                action: action.id().to_owned(),
                source,
            })?;

        if !schema.is_object() {
            return Err(ExtensionError::SchemaNotObject(action.id().to_owned()));
        }

        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_actions::Risk;

    use super::ActionSpec;
    use crate::interface::DescribedAction;

    fn action(id: &str, schema: &str) -> DescribedAction {
        DescribedAction::new(
            id.to_owned(),
            "Greet a host".to_owned(),
            Risk::Privileged,
            true,
            schema.to_owned(),
        )
    }

    #[test]
    fn accepts_actions_named_after_the_extension() {
        let spec = ActionSpec::from_described("hello", &action("hello.greet", "{}")).unwrap();

        assert_eq!(spec.id().as_str(), "hello.greet");
        assert_eq!(spec.risk(), Risk::Privileged);
        assert!(spec.takes_target());
        assert_eq!(spec.params_schema(), &json!({}));
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
