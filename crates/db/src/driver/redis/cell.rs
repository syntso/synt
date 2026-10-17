use redis::Value as RedisValue;
use serde_json::{Value, json};

use crate::json_cell::JsonCell;

const OKAY: &str = "OK";

pub(super) struct Cell(RedisValue);

impl Cell {
    pub(super) fn new(value: RedisValue) -> Self {
        Self(value)
    }

    pub(super) fn json(value: RedisValue) -> Value {
        Self::new(value).into()
    }

    fn list(items: Vec<RedisValue>) -> Value {
        Value::Array(items.into_iter().map(Self::json).collect())
    }

    fn pair((key, value): (RedisValue, RedisValue)) -> Value {
        Value::Array(vec![Self::json(key), Self::json(value)])
    }
}

impl From<Cell> for Value {
    fn from(cell: Cell) -> Self {
        match cell.0 {
            RedisValue::Nil => Self::Null,
            RedisValue::Int(number) => json!(number),
            RedisValue::BulkString(bytes) | RedisValue::BigNumber(bytes) => {
                JsonCell::text_or_binary(&bytes).into()
            }
            RedisValue::Array(items) | RedisValue::Set(items) => Cell::list(items),
            RedisValue::SimpleString(text) | RedisValue::VerbatimString { text, .. } => {
                Self::String(text)
            }
            RedisValue::Okay => Self::String(OKAY.to_owned()),
            RedisValue::Map(pairs) => Self::Array(pairs.into_iter().map(Cell::pair).collect()),
            RedisValue::Attribute { data, .. } => Cell::json(*data),
            RedisValue::Double(number) => JsonCell::float(number).into(),
            RedisValue::Boolean(flag) => Self::Bool(flag),
            RedisValue::Push { data, .. } => Cell::list(data),
            RedisValue::ServerError(error) => json!({ "error": error.to_string() }),
            other => Self::String(format!("{other:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use redis::Value as RedisValue;
    use serde_json::json;

    use super::Cell;

    #[test]
    fn nested_replies_keep_their_shape() {
        let reply = RedisValue::Array(vec![
            RedisValue::BulkString(b"0".to_vec()),
            RedisValue::Array(vec![RedisValue::Okay]),
        ]);

        assert_eq!(Cell::json(reply), json!(["0", ["OK"]]));
    }
}
