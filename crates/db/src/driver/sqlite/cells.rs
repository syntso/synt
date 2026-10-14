use rusqlite::types::{Value as Cell, ValueRef};
use serde_json::{Value, json};

use crate::cells::{binary, float, text_or_binary};

pub(super) fn to_json(cell: ValueRef<'_>) -> Value {
    match cell {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(number) => json!(number),
        ValueRef::Real(number) => float(number),
        ValueRef::Text(bytes) => text_or_binary(bytes),
        ValueRef::Blob(bytes) => binary(bytes),
    }
}

pub(super) fn from_json(value: &Value) -> Cell {
    match value {
        Value::Null => Cell::Null,
        Value::Bool(flag) => Cell::Integer(i64::from(*flag)),
        Value::Number(number) => number
            .as_i64()
            .map(Cell::Integer)
            .or_else(|| number.as_f64().map(Cell::Real))
            .unwrap_or(Cell::Null),
        Value::String(text) => Cell::Text(text.clone()),
        other => Cell::Text(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::types::{Value as Cell, ValueRef};
    use serde_json::json;

    use super::{from_json, to_json};

    #[test]
    fn storage_classes_map_to_json() {
        assert_eq!(to_json(ValueRef::Integer(7)), json!(7));
        assert_eq!(to_json(ValueRef::Real(0.5)), json!(0.5));
        assert_eq!(to_json(ValueRef::Text(b"hi")), json!("hi"));
        assert_eq!(to_json(ValueRef::Blob(&[1, 2])), json!("\\x0102"));
        assert_eq!(to_json(ValueRef::Null), json!(null));
    }

    #[test]
    fn json_params_map_to_storage_classes() {
        assert_eq!(from_json(&json!(true)), Cell::Integer(1));
        assert_eq!(from_json(&json!(2.5)), Cell::Real(2.5));
        assert_eq!(from_json(&json!("x")), Cell::Text("x".to_owned()));
        assert_eq!(from_json(&json!([1])), Cell::Text("[1]".to_owned()));
    }
}
