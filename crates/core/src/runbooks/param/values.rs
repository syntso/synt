use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use super::ParamSpec;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Values {
    values: BTreeMap<String, Value>,
}

impl Values {
    pub(crate) fn resolve(specs: &[ParamSpec], given: &Map<String, Value>) -> Result<Self, String> {
        if let Some(unknown) = given
            .keys()
            .find(|name| !specs.iter().any(|spec| spec.name == **name))
        {
            return Err(format!("the runbook has no param {unknown}"));
        }

        let mut values = BTreeMap::new();

        for spec in specs {
            let value = given
                .get(&spec.name)
                .or(spec.default.as_ref())
                .ok_or_else(|| format!("param {} is required", spec.name))?;

            let accepted = spec
                .accept(value)
                .map_err(|reason| format!("param {}: {reason}", spec.name))?;

            values.insert(spec.name.clone(), accepted);
        }

        Ok(Self { values })
    }

    #[cfg(test)]
    pub(crate) fn from_pairs<const N: usize>(pairs: [(&str, Value); N]) -> Self {
        let values = pairs
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect();

        Self { values }
    }

    pub(crate) fn get(&self, name: &str) -> Result<&Value, String> {
        self.values
            .get(name)
            .ok_or_else(|| format!("no value for param {name}"))
    }

    pub(crate) fn text(&self, name: &str) -> Result<String, String> {
        match self.get(name)? {
            Value::String(text) => Ok(text.clone()),
            other => Ok(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::Values;
    use crate::runbooks::param::{ParamKind, ParamSpec};

    fn specs() -> Vec<ParamSpec> {
        vec![
            ParamSpec::literal("host".to_owned(), ParamKind::Target, "@orders/api-1"),
            ParamSpec {
                default: None,
                ..ParamSpec::literal("attempts".to_owned(), ParamKind::Integer, "")
            },
        ]
    }

    fn given(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn given_values_win_over_defaults_and_are_normalized() {
        let values = Values::resolve(&specs(), &given(&json!({ "attempts": "3" }))).unwrap();

        assert_eq!(values.get("host"), Ok(&json!("@orders/api-1")));
        assert_eq!(values.get("attempts"), Ok(&json!(3)));
        assert_eq!(values.text("attempts"), Ok("3".to_owned()));
    }

    #[test]
    fn a_param_without_a_default_must_be_given() {
        let error = Values::resolve(&specs(), &Map::new()).unwrap_err();

        assert_eq!(error, "param attempts is required");
    }

    #[test]
    fn unknown_params_are_refused() {
        let error = Values::resolve(&specs(), &given(&json!({ "attempts": 1, "force": true })))
            .unwrap_err();

        assert_eq!(error, "the runbook has no param force");
    }

    #[test]
    fn values_are_checked_against_their_kind() {
        let error = Values::resolve(
            &specs(),
            &given(&json!({ "attempts": 1, "host": "api-1; reboot" })),
        )
        .unwrap_err();

        assert!(error.starts_with("param host:"), "{error}");
    }
}
