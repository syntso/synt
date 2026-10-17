use data_encoding::HEXLOWER;
use serde_json::{Number, Value};

const BINARY_PREFIX: &str = "\\x";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonCell(Value);

impl JsonCell {
    pub(crate) fn binary(bytes: &[u8]) -> Self {
        Self(Value::String(format!(
            "{BINARY_PREFIX}{}",
            HEXLOWER.encode(bytes)
        )))
    }

    pub(crate) fn text_or_binary(bytes: &[u8]) -> Self {
        match std::str::from_utf8(bytes) {
            Ok(text) => Self(Value::String(text.to_owned())),
            Err(_) => Self::binary(bytes),
        }
    }

    pub(crate) fn float(number: f64) -> Self {
        let value = Number::from_f64(number)
            .map_or_else(|| Value::String(number.to_string()), Value::Number);

        Self(value)
    }
}

impl From<JsonCell> for Value {
    fn from(cell: JsonCell) -> Self {
        cell.0
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::JsonCell;

    fn value(cell: JsonCell) -> Value {
        cell.into()
    }

    #[test]
    fn bytes_are_shown_as_hex_like_postgres_does() {
        assert_eq!(
            value(JsonCell::binary(&[0xde, 0xad, 0x01])),
            json!("\\xdead01")
        );
    }

    #[test]
    fn utf8_bytes_are_text_and_others_are_hex() {
        assert_eq!(value(JsonCell::text_or_binary(b"plain")), json!("plain"));
        assert_eq!(
            value(JsonCell::text_or_binary(&[0xff, 0x00])),
            json!("\\xff00")
        );
    }

    #[test]
    fn floats_json_cannot_hold_become_text() {
        assert_eq!(value(JsonCell::float(1.5)), json!(1.5));
        assert_eq!(value(JsonCell::float(f64::NAN)), json!("NaN"));
        assert_eq!(value(JsonCell::float(f64::INFINITY)), json!("inf"));
    }
}
