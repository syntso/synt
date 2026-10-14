use serde_json::Value;

use super::ViewCheck;

const MOST_OPERATIONS: usize = 1_000;
const OPERATIONS: [&str; 3] = ["add", "remove", "replace"];

pub(crate) struct ViewPatch;

impl ViewPatch {
    pub(crate) fn check(patch: &Value, owns: &dyn Fn(&str) -> bool) -> Result<(), String> {
        let operations = patch
            .as_array()
            .ok_or("a view patch is a list of JSON Patch operations")?;

        if operations.is_empty() || operations.len() > MOST_OPERATIONS {
            return Err(format!(
                "a view patch holds 1 to {MOST_OPERATIONS} operations"
            ));
        }

        operations
            .iter()
            .try_for_each(|operation| Self::operation(operation, owns))
    }

    fn operation(operation: &Value, owns: &dyn Fn(&str) -> bool) -> Result<(), String> {
        let op = operation
            .get("op")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let path = operation.get("path").and_then(Value::as_str);

        if !OPERATIONS.contains(&op) {
            return Err(format!("view patches use only {}", OPERATIONS.join(", ")));
        }

        if !path.is_some_and(|path| path.is_empty() || path.starts_with('/')) {
            return Err("every view patch operation needs a JSON pointer path".to_owned());
        }

        match (op, operation.get("value")) {
            ("remove", _) => Ok(()),
            (_, Some(value)) => ViewCheck::new(owns).value(value),
            (_, None) => Err(format!("a view patch {op} needs a value")),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ViewPatch;

    fn owns(action: &str) -> bool {
        action.starts_with("uptime.")
    }

    #[test]
    fn accepts_replacing_values_and_nodes() {
        let patch = json!([
            { "op": "replace", "path": "/children/0/text", "value": "load 0.42" },
            { "op": "add", "path": "/children/-", "value": { "type": "text", "text": "new" } },
            { "op": "remove", "path": "/children/2" }
        ]);

        assert_eq!(ViewPatch::check(&patch, &owns), Ok(()));
    }

    #[test]
    fn refuses_other_operations_and_foreign_buttons() {
        for patch in [
            json!({ "op": "replace" }),
            json!([]),
            json!([{ "op": "move", "from": "/a", "path": "/b" }]),
            json!([{ "op": "replace", "path": "nope", "value": 1 }]),
            json!([{ "op": "add", "path": "/x" }]),
            json!([{ "op": "add", "path": "/x", "value": { "type": "button", "title": "Go", "action": "ssh.exec" } }]),
        ] {
            assert!(ViewPatch::check(&patch, &owns).is_err(), "{patch}");
        }
    }
}
