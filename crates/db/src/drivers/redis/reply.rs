use redis::Value as RedisValue;
use serde_json::{Value, json};

use crate::cells::{float, text_or_binary};

const VALUE: &str = "value";
const KEY: &str = "key";
const OKAY: &str = "OK";

pub(super) struct Reply {
    value: RedisValue,
}

impl Reply {
    pub(super) fn new(value: RedisValue) -> Self {
        Self { value }
    }

    pub(super) fn into_rows(self) -> (Vec<String>, Vec<Vec<Value>>) {
        match self.value {
            RedisValue::Array(items) | RedisValue::Set(items) => (
                vec![VALUE.to_owned()],
                items.into_iter().map(|item| vec![json_of(item)]).collect(),
            ),
            RedisValue::Map(pairs) => (
                vec![KEY.to_owned(), VALUE.to_owned()],
                pairs
                    .into_iter()
                    .map(|(key, value)| vec![json_of(key), json_of(value)])
                    .collect(),
            ),
            single => (vec![VALUE.to_owned()], vec![vec![json_of(single)]]),
        }
    }

    pub(super) fn into_json(self) -> Value {
        match self.value {
            RedisValue::Nil => Value::Null,
            RedisValue::Int(number) => json!(number),
            RedisValue::BulkString(bytes) | RedisValue::BigNumber(bytes) => text_or_binary(&bytes),
            RedisValue::Array(items) | RedisValue::Set(items) => list(items),
            RedisValue::SimpleString(text) | RedisValue::VerbatimString { text, .. } => {
                Value::String(text)
            }
            RedisValue::Okay => Value::String(OKAY.to_owned()),
            RedisValue::Map(pairs) => Value::Array(pairs.into_iter().map(pair).collect()),
            RedisValue::Attribute { data, .. } => json_of(*data),
            RedisValue::Double(number) => float(number),
            RedisValue::Boolean(flag) => Value::Bool(flag),
            RedisValue::Push { data, .. } => list(data),
            RedisValue::ServerError(error) => json!({ "error": error.to_string() }),
            other => Value::String(format!("{other:?}")),
        }
    }
}

fn json_of(value: RedisValue) -> Value {
    Reply::new(value).into_json()
}

fn list(items: Vec<RedisValue>) -> Value {
    Value::Array(items.into_iter().map(json_of).collect())
}

fn pair((key, value): (RedisValue, RedisValue)) -> Value {
    Value::Array(vec![json_of(key), json_of(value)])
}

#[cfg(test)]
mod tests {
    use redis::Value as RedisValue;
    use serde_json::{Value, json};

    use super::Reply;

    fn rows(reply: RedisValue) -> (Vec<String>, Vec<Vec<Value>>) {
        Reply::new(reply).into_rows()
    }

    fn to_json(reply: RedisValue) -> Value {
        Reply::new(reply).into_json()
    }

    #[test]
    fn scalar_replies_become_one_row() {
        let (columns, rows) = rows(RedisValue::BulkString(b"ann".to_vec()));

        assert_eq!(columns, ["value"]);
        assert_eq!(rows, vec![vec![json!("ann")]]);
    }

    #[test]
    fn list_replies_become_a_row_per_item() {
        let reply = RedisValue::Array(vec![RedisValue::Int(1), RedisValue::Nil]);

        assert_eq!(rows(reply).1, vec![vec![json!(1)], vec![json!(null)]]);
    }

    #[test]
    fn map_replies_become_key_and_value_rows() {
        let reply = RedisValue::Map(vec![(
            RedisValue::SimpleString("a".to_owned()),
            RedisValue::Int(1),
        )]);

        let (columns, rows) = rows(reply);

        assert_eq!(columns, ["key", "value"]);
        assert_eq!(rows, vec![vec![json!("a"), json!(1)]]);
    }

    #[test]
    fn nested_replies_keep_their_shape() {
        let reply = RedisValue::Array(vec![
            RedisValue::BulkString(b"0".to_vec()),
            RedisValue::Array(vec![RedisValue::Okay]),
        ]);

        assert_eq!(to_json(reply), json!(["0", ["OK"]]));
    }
}
