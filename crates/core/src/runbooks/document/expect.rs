mod pointer;

pub use pointer::JsonPointer;

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::CoreError;
use crate::runbooks::param::Values;
use crate::runbooks::template::TextTemplate;

const EXIT: &str = "exit";
const STDOUT: &str = "stdout";
const SIGNAL: &str = "signal";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exit: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    contains: Option<String>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<JsonPointer, Value>,
}

impl Expect {
    pub(crate) fn contains(&self) -> Option<&str> {
        self.contains.as_deref()
    }

    pub(crate) fn fields(&self) -> impl Iterator<Item = (&JsonPointer, &Value)> {
        self.fields.iter()
    }

    pub(crate) fn exit_wanted(&self) -> i64 {
        self.exit.unwrap_or(0)
    }

    pub(crate) fn render(&self, values: &Values) -> Result<Self, CoreError> {
        let contains = self
            .contains
            .as_deref()
            .map(|text| TextTemplate::parse(text).render_text(values))
            .transpose()?;

        Ok(Self {
            contains,
            ..self.clone()
        })
    }

    pub(crate) fn judge(&self, result: &Value) -> Result<(), CoreError> {
        self.judge_exit(result)?;
        self.judge_output(result)?;

        for (pointer, wanted) in &self.fields {
            Self::judge_field(result, pointer, wanted)?;
        }

        Ok(())
    }

    fn judge_exit(&self, result: &Value) -> Result<(), CoreError> {
        let carried = result.get(EXIT);

        let wanted = match (self.exit, carried) {
            (None, None) => return Ok(()),
            (Some(wanted), None) => return Err(CoreError::NoExitCarried(wanted)),
            (_, Some(_)) => self.exit_wanted(),
        };

        match carried.and_then(Value::as_i64) {
            Some(exit) if exit == wanted => Ok(()),
            Some(exit) => Err(CoreError::WrongExit { exit, wanted }),
            None => Err(CoreError::NoExitCode {
                wanted,
                signal: result
                    .get(SIGNAL)
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            }),
        }
    }

    fn judge_output(&self, result: &Value) -> Result<(), CoreError> {
        let Some(wanted) = &self.contains else {
            return Ok(());
        };

        let output = result
            .get(STDOUT)
            .and_then(Value::as_str)
            .unwrap_or_default();

        if output.contains(wanted.as_str()) {
            return Ok(());
        }

        Err(CoreError::OutputLacks(wanted.clone()))
    }

    fn judge_field(result: &Value, pointer: &JsonPointer, wanted: &Value) -> Result<(), CoreError> {
        let pointer_text = pointer.as_str();

        match result.pointer(pointer_text) {
            Some(found) if found == wanted => Ok(()),
            Some(found) => Err(CoreError::WrongField {
                pointer: pointer_text.to_owned(),
                found: found.clone(),
                wanted: wanted.clone(),
            }),
            None => Err(CoreError::ResultLacks {
                pointer: pointer_text.to_owned(),
                wanted: wanted.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{Expect, JsonPointer};

    fn judged(expect: &Expect, result: &Value) -> Result<(), String> {
        expect.judge(result).map_err(|error| error.to_string())
    }

    fn fields(pointer: &str, wanted: Value) -> Expect {
        Expect {
            fields: [(JsonPointer::new(pointer).unwrap(), wanted)].into(),
            ..Expect::default()
        }
    }

    #[test]
    fn a_result_with_an_exit_must_exit_zero_unless_told_otherwise() {
        let nothing = Expect::default();

        assert_eq!(judged(&nothing, &json!({ "exit": 0 })), Ok(()));
        assert_eq!(
            judged(&nothing, &json!({ "exit": 3 })),
            Err("exit 3, expected 0".to_owned())
        );
        assert_eq!(judged(&nothing, &json!({ "files": 2 })), Ok(()));
    }

    #[test]
    fn an_expected_exit_replaces_zero() {
        let one = Expect {
            exit: Some(1),
            ..Expect::default()
        };

        assert_eq!(judged(&one, &json!({ "exit": 1 })), Ok(()));
        assert!(one.judge(&json!({ "exit": 0 })).is_err());
        assert!(one.judge(&json!({ "value": 1 })).is_err());
    }

    #[test]
    fn a_signal_is_named_when_no_exit_came() {
        let error = judged(
            &Expect::default(),
            &json!({ "exit": null, "signal": "KILL" }),
        );

        assert_eq!(
            error,
            Err("no exit code, expected 0 (ended by signal KILL)".to_owned())
        );
    }

    #[test]
    fn the_output_can_be_required_to_contain_text() {
        let healthy = Expect {
            contains: Some("ok".to_owned()),
            ..Expect::default()
        };

        assert_eq!(
            judged(&healthy, &json!({ "exit": 0, "stdout": "status: ok\n" })),
            Ok(())
        );
        assert!(
            healthy
                .judge(&json!({ "exit": 0, "stdout": "down" }))
                .is_err()
        );
    }

    #[test]
    fn fields_are_found_by_json_pointer() {
        let running = fields("/state", json!("up"));

        assert_eq!(judged(&running, &json!({ "state": "up" })), Ok(()));
        assert_eq!(
            judged(&running, &json!({ "state": "warming" })),
            Err("/state is \"warming\", expected \"up\"".to_owned())
        );
        assert!(running.judge(&json!({})).is_err());
    }

    #[test]
    fn field_names_must_be_pointers() {
        let loose: Result<Expect, _> =
            serde_json::from_value(json!({ "fields": { "state": "up" } }));

        assert!(loose.is_err());
    }
}
