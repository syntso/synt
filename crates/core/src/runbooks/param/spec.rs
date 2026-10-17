use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ParamKind, ParamName};

use crate::CoreError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParamSpec {
    name: ParamName,
    kind: ParamKind,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    default: Option<Value>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    choices: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    min: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    max: Option<i64>,
}

impl ParamSpec {
    pub(crate) fn literal(name: ParamName, kind: ParamKind, default: &str) -> Self {
        Self {
            name,
            kind,
            default: Some(Value::from(default)),
            description: String::new(),
            choices: Vec::new(),
            min: None,
            max: None,
        }
    }

    pub(crate) fn name(&self) -> &ParamName {
        &self.name
    }

    pub(crate) fn default(&self) -> Option<&Value> {
        self.default.as_ref()
    }

    pub(crate) fn default_text(&self) -> Option<String> {
        self.default.as_ref().map(|default| match default {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        })
    }

    pub(crate) fn check(&self) -> Result<(), CoreError> {
        self.check_choices()?;
        self.check_bounds()?;

        match &self.default {
            Some(default) => self.accept(default).map(|_| ()),
            None => Ok(()),
        }
    }

    pub(crate) fn accept(&self, value: &Value) -> Result<Value, CoreError> {
        let accepted = self.kind.accept(value)?;

        self.within_choices(&accepted)?;
        self.within_bounds(&accepted)?;

        Ok(accepted)
    }

    fn check_choices(&self) -> Result<(), CoreError> {
        let is_choice = self.kind == ParamKind::Choice;

        if is_choice && self.choices.is_empty() {
            return Err(CoreError::NoChoices(self.name.clone()));
        }

        if !is_choice && !self.choices.is_empty() {
            return Err(CoreError::StrayChoices(self.name.clone()));
        }

        Ok(())
    }

    fn check_bounds(&self) -> Result<(), CoreError> {
        let bounded = self.min.is_some() || self.max.is_some();

        if bounded && self.kind != ParamKind::Integer {
            return Err(CoreError::StrayBounds(self.name.clone()));
        }

        if let (Some(min), Some(max)) = (self.min, self.max)
            && min > max
        {
            return Err(CoreError::UpsideDown {
                name: self.name.clone(),
                min,
                max,
            });
        }

        Ok(())
    }

    fn within_choices(&self, value: &Value) -> Result<(), CoreError> {
        if self.kind != ParamKind::Choice {
            return Ok(());
        }

        let chosen = value.as_str().unwrap_or_default();

        if self.choices.iter().any(|choice| choice == chosen) {
            return Ok(());
        }

        Err(CoreError::NotAChoice {
            chosen: chosen.to_owned(),
            choices: self.choices.clone(),
        })
    }

    fn within_bounds(&self, value: &Value) -> Result<(), CoreError> {
        let Some(number) = value.as_i64() else {
            return Ok(());
        };

        if self.min.is_some_and(|min| number < min) || self.max.is_some_and(|max| number > max) {
            return Err(CoreError::OutOfBounds {
                number,
                min: self.min,
                max: self.max,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ParamSpec;
    use crate::runbooks::param::{ParamKind, ParamName};

    fn spec(kind: ParamKind) -> ParamSpec {
        ParamSpec {
            name: ParamName::new("value").unwrap(),
            kind,
            default: None,
            description: String::new(),
            choices: Vec::new(),
            min: None,
            max: None,
        }
    }

    #[test]
    fn a_choice_accepts_only_its_choices() {
        let choice = ParamSpec {
            choices: vec!["blue".to_owned(), "green".to_owned()],
            ..spec(ParamKind::Choice)
        };

        assert_eq!(choice.accept(&json!("green")).unwrap(), json!("green"));
        assert!(
            choice
                .accept(&json!("red"))
                .unwrap_err()
                .to_string()
                .contains("blue, green")
        );
        assert!(choice.check().is_ok());
    }

    #[test]
    fn an_integer_stays_within_its_bounds() {
        let bounded = ParamSpec {
            min: Some(1),
            max: Some(10),
            ..spec(ParamKind::Integer)
        };

        assert_eq!(bounded.accept(&json!("10")).unwrap(), json!(10));
        assert_eq!(
            bounded.accept(&json!(0)).unwrap_err().to_string(),
            "0 is outside 1..10"
        );
        assert!(bounded.accept(&json!(11)).is_err());
    }

    #[test]
    fn a_spec_is_checked_as_a_whole() {
        let empty_choice = spec(ParamKind::Choice);
        let stray_choices = ParamSpec {
            choices: vec!["a".to_owned()],
            ..spec(ParamKind::Word)
        };
        let upside_down = ParamSpec {
            min: Some(5),
            max: Some(1),
            ..spec(ParamKind::Integer)
        };
        let text_bounds = ParamSpec {
            max: Some(1),
            ..spec(ParamKind::String)
        };
        let bad_default = ParamSpec {
            default: Some(json!("a b")),
            ..spec(ParamKind::Word)
        };

        for broken in [
            empty_choice,
            stray_choices,
            upside_down,
            text_bounds,
            bad_default,
        ] {
            assert!(broken.check().is_err(), "{broken:?} passed");
        }
    }

    #[test]
    fn a_name_is_checked_as_it_is_read() {
        let unnamed: Result<ParamSpec, _> =
            serde_json::from_value(json!({ "name": "dry-run", "kind": "boolean" }));

        assert!(unnamed.unwrap_err().to_string().contains("\"dry-run\""));
    }
}
