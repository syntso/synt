use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ParamKind, is_param_name};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParamSpec {
    pub name: String,
    pub kind: ParamKind,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
}

impl ParamSpec {
    pub(crate) fn literal(name: String, kind: ParamKind, default: &str) -> Self {
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

    pub(crate) fn check(&self) -> Result<(), String> {
        if !is_param_name(&self.name) {
            return Err(format!(
                "param {:?}: a name starts with a letter or _ and holds only letters, digits and _",
                self.name
            ));
        }

        self.check_choices()?;
        self.check_bounds()?;

        match &self.default {
            Some(default) => self.accept(default).map(|_| ()),
            None => Ok(()),
        }
    }

    pub(crate) fn accept(&self, value: &Value) -> Result<Value, String> {
        let accepted = self.kind.accept(value)?;

        self.within_choices(&accepted)?;
        self.within_bounds(&accepted)?;

        Ok(accepted)
    }

    fn check_choices(&self) -> Result<(), String> {
        let is_choice = self.kind == ParamKind::Choice;

        if is_choice && self.choices.is_empty() {
            return Err(format!("param {}: a choice needs choices", self.name));
        }

        if !is_choice && !self.choices.is_empty() {
            return Err(format!("param {}: only a choice has choices", self.name));
        }

        Ok(())
    }

    fn check_bounds(&self) -> Result<(), String> {
        let bounded = self.min.is_some() || self.max.is_some();

        if bounded && self.kind != ParamKind::Integer {
            return Err(format!(
                "param {}: only an integer has min and max",
                self.name
            ));
        }

        if let (Some(min), Some(max)) = (self.min, self.max)
            && min > max
        {
            return Err(format!("param {}: min {min} is above max {max}", self.name));
        }

        Ok(())
    }

    fn within_choices(&self, value: &Value) -> Result<(), String> {
        if self.kind != ParamKind::Choice {
            return Ok(());
        }

        let chosen = value.as_str().unwrap_or_default();

        if self.choices.iter().any(|choice| choice == chosen) {
            return Ok(());
        }

        Err(format!(
            "{chosen:?} is not one of {}",
            self.choices.join(", ")
        ))
    }

    fn within_bounds(&self, value: &Value) -> Result<(), String> {
        let Some(number) = value.as_i64() else {
            return Ok(());
        };

        if self.min.is_some_and(|min| number < min) || self.max.is_some_and(|max| number > max) {
            return Err(format!(
                "{number} is outside {}..{}",
                self.min.map(|min| min.to_string()).unwrap_or_default(),
                self.max.map(|max| max.to_string()).unwrap_or_default(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ParamSpec;
    use crate::runbooks::param::ParamKind;

    fn spec(kind: ParamKind) -> ParamSpec {
        ParamSpec {
            name: "value".to_owned(),
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

        assert_eq!(choice.accept(&json!("green")), Ok(json!("green")));
        assert!(
            choice
                .accept(&json!("red"))
                .unwrap_err()
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

        assert_eq!(bounded.accept(&json!("10")), Ok(json!(10)));
        assert!(bounded.accept(&json!(0)).is_err());
        assert!(bounded.accept(&json!(11)).is_err());
    }

    #[test]
    fn a_spec_is_checked_as_a_whole() {
        let unnamed = ParamSpec {
            name: "dry-run".to_owned(),
            ..spec(ParamKind::Boolean)
        };
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
            unnamed,
            empty_choice,
            stray_choices,
            upside_down,
            text_bounds,
            bad_default,
        ] {
            assert!(broken.check().is_err(), "{broken:?} passed");
        }
    }
}
