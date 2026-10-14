use serde_json::{Value, json};
use synt_db::masked;

const SQL: &str = "sql";
const COMMAND: &str = "command";
const PARAMS: &str = "params";
const HIDDEN_ARGUMENT: &str = "?";

pub(super) fn recorded_statement(params: &Value) -> Value {
    let Some(fields) = params.as_object() else {
        return params.clone();
    };

    let mut recorded = fields.clone();

    if let Some(Value::String(sql)) = fields.get(SQL) {
        recorded.insert(SQL.to_owned(), Value::String(masked(sql)));
    }

    if let Some(command) = fields.get(COMMAND) {
        recorded.insert(COMMAND.to_owned(), hidden_arguments(command));
    }

    if let Some(given) = fields.get(PARAMS) {
        let count = given.as_array().map_or(0, Vec::len);
        recorded.insert(PARAMS.to_owned(), json!(count));
    }

    Value::Object(recorded)
}

fn hidden_arguments(command: &Value) -> Value {
    let Some(words) = command.as_array() else {
        return Value::String(HIDDEN_ARGUMENT.to_owned());
    };

    let shown = words
        .iter()
        .enumerate()
        .map(|(index, word)| match (index, word) {
            (0, Value::String(name)) => Value::String(name.clone()),
            _ => Value::String(HIDDEN_ARGUMENT.to_owned()),
        });

    Value::Array(shown.collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::recorded_statement;

    #[test]
    fn sql_literals_are_masked_and_parameters_only_counted() {
        let params = json!({
            "sql": "SELECT * FROM users WHERE email = 'ann@example.com' AND id = $1",
            "params": ["4242", 7],
            "limit": 10,
        });

        assert_eq!(
            recorded_statement(&params),
            json!({
                "sql": "SELECT * FROM users WHERE email = ? AND id = $1",
                "params": 2,
                "limit": 10,
            })
        );
    }

    #[test]
    fn redis_commands_keep_only_their_name() {
        let params = json!({ "command": ["SET", "session:ann", "token-abc"] });

        assert_eq!(
            recorded_statement(&params),
            json!({ "command": ["SET", "?", "?"] })
        );
    }

    #[test]
    fn something_that_is_not_an_object_is_left_alone() {
        assert_eq!(recorded_statement(&json!(null)), json!(null));
    }
}
