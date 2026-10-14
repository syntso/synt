use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use super::{ParamName, ParamSpec};
use crate::runbooks::RunbookError;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct Values {
    values: BTreeMap<ParamName, Value>,
}

impl Values {
    pub(crate) fn resolve(
        specs: &[ParamSpec],
        given: &Map<String, Value>,
    ) -> Result<Self, RunbookError> {
        let unknown = given.keys().find(|name| {
            !specs
                .iter()
                .any(|spec| spec.name().as_str() == name.as_str())
        });

        if let Some(unknown) = unknown {
            return Err(RunbookError::UnknownParam(unknown.clone()));
        }

        let values = specs
            .iter()
            .map(|spec| Ok((spec.name().clone(), Self::value_of(spec, given)?)))
            .collect::<Result<_, RunbookError>>()?;

        Ok(Self { values })
    }

    #[cfg(test)]
    pub(crate) fn from_pairs<const N: usize>(pairs: [(&str, Value); N]) -> Self {
        let values = pairs
            .into_iter()
            .map(|(name, value)| (ParamName::new(name).unwrap(), value))
            .collect();

        Self { values }
    }

    pub(crate) fn get(&self, name: &str) -> Result<&Value, RunbookError> {
        self.values
            .get(name)
            .ok_or_else(|| RunbookError::NoValue(name.to_owned()))
    }

    pub(crate) fn text(&self, name: &str) -> Result<String, RunbookError> {
        match self.get(name)? {
            Value::String(text) => Ok(text.clone()),
            other => Ok(other.to_string()),
        }
    }

    fn value_of(spec: &ParamSpec, given: &Map<String, Value>) -> Result<Value, RunbookError> {
        let name = spec.name();

        let value = given
            .get(name.as_str())
            .or(spec.default())
            .ok_or_else(|| RunbookError::MissingParam(name.clone()))?;

        spec.accept(value).map_err(|error| error.in_param(name))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use super::Values;
    use crate::runbooks::param::{ParamKind, ParamName, ParamSpec};

    fn specs() -> Vec<ParamSpec> {
        let attempts: ParamSpec =
            serde_json::from_value(json!({ "name": "attempts", "kind": "integer" })).unwrap();
        let host = ParamSpec::literal(
            ParamName::new("host").unwrap(),
            ParamKind::Target,
            "@orders/api-1",
        );

        vec![host, attempts]
    }

    fn given(value: &Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    fn refusal(given: &Map<String, Value>) -> String {
        Values::resolve(&specs(), given).unwrap_err().to_string()
    }

    #[test]
    fn given_values_win_over_defaults_and_are_normalized() {
        let values = Values::resolve(&specs(), &given(&json!({ "attempts": "3" }))).unwrap();

        assert_eq!(values.get("host").unwrap(), &json!("@orders/api-1"));
        assert_eq!(values.get("attempts").unwrap(), &json!(3));
        assert_eq!(values.text("attempts").unwrap(), "3");
    }

    #[test]
    fn a_param_without_a_default_must_be_given() {
        assert_eq!(refusal(&Map::new()), "param attempts is required");
    }

    #[test]
    fn unknown_params_are_refused() {
        assert_eq!(
            refusal(&given(&json!({ "attempts": 1, "force": true }))),
            "the runbook has no param force"
        );
    }

    #[test]
    fn values_are_checked_against_their_kind() {
        let error = refusal(&given(&json!({ "attempts": 1, "host": "api-1; reboot" })));

        assert!(error.starts_with("param host:"), "{error}");
    }
}
