use serde_json::{Map, Number, Value};

use super::DetailsError;
use crate::codec::{CodecError, Decoder, Encoder};

pub(super) const MAX_DEPTH: usize = 32;

pub(super) const NULL: u8 = 0;
const FALSE: u8 = 1;
const TRUE: u8 = 2;
const SIGNED: u8 = 3;
const UNSIGNED: u8 = 4;
const TEXT: u8 = 5;
const LIST: u8 = 6;
const MAP: u8 = 7;

pub(super) fn encode(value: &Value, out: &mut Vec<u8>) -> Result<(), DetailsError> {
    write(value, &mut Encoder::new(out), 0)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Value, CodecError> {
    let mut decoder = Decoder::new(bytes);
    let value = read(&mut decoder, 0)?;

    decoder.finish()?;
    Ok(value)
}

fn write(value: &Value, encoder: &mut Encoder<'_>, depth: usize) -> Result<(), DetailsError> {
    if depth > MAX_DEPTH {
        return Err(DetailsError::TooDeep { limit: MAX_DEPTH });
    }

    match value {
        Value::Null => encoder.u8(NULL),
        Value::Bool(false) => encoder.u8(FALSE),
        Value::Bool(true) => encoder.u8(TRUE),
        Value::Number(number) => write_number(number, encoder)?,
        Value::String(text) => {
            encoder.u8(TEXT);
            encoder.str(text);
        }
        Value::Array(items) => {
            encoder.u8(LIST);
            encoder.len(items.len());

            for item in items {
                write(item, encoder, depth + 1)?;
            }
        }
        Value::Object(entries) => {
            let mut sorted: Vec<_> = entries.iter().collect();
            sorted.sort_unstable_by_key(|(key, _)| *key);

            encoder.u8(MAP);
            encoder.len(sorted.len());

            for (key, value) in sorted {
                encoder.str(key);
                write(value, encoder, depth + 1)?;
            }
        }
    }

    Ok(())
}

fn write_number(number: &Number, encoder: &mut Encoder<'_>) -> Result<(), DetailsError> {
    if let Some(value) = number.as_i64() {
        encoder.u8(SIGNED);
        encoder.i64(value);
    } else if let Some(value) = number.as_u64() {
        encoder.u8(UNSIGNED);
        encoder.u64(value);
    } else {
        return Err(DetailsError::Float);
    }

    Ok(())
}

fn read(decoder: &mut Decoder<'_>, depth: usize) -> Result<Value, CodecError> {
    if depth > MAX_DEPTH {
        return Err(CodecError("details nest too deep"));
    }

    let value = match decoder.u8()? {
        NULL => Value::Null,
        FALSE => Value::Bool(false),
        TRUE => Value::Bool(true),
        SIGNED => Value::from(decoder.i64()?),
        UNSIGNED => read_unsigned(decoder)?,
        TEXT => Value::String(decoder.str()?.to_owned()),
        LIST => read_list(decoder, depth)?,
        MAP => read_map(decoder, depth)?,
        _ => return Err(CodecError("unknown value tag")),
    };

    Ok(value)
}

fn read_unsigned(decoder: &mut Decoder<'_>) -> Result<Value, CodecError> {
    let value = decoder.u64()?;

    if i64::try_from(value).is_ok() {
        return Err(CodecError("small unsigned number must be stored as signed"));
    }

    Ok(Value::from(value))
}

fn read_list(decoder: &mut Decoder<'_>, depth: usize) -> Result<Value, CodecError> {
    let len = decoder.len()?;
    let mut items = Vec::new();

    for _ in 0..len {
        items.push(read(decoder, depth + 1)?);
    }

    Ok(Value::Array(items))
}

fn read_map(decoder: &mut Decoder<'_>, depth: usize) -> Result<Value, CodecError> {
    let len = decoder.len()?;
    let mut entries = Map::new();
    let mut previous: Option<&str> = None;

    for _ in 0..len {
        let key = decoder.str()?;

        if previous.is_some_and(|previous| previous >= key) {
            return Err(CodecError("map keys out of order"));
        }

        let value = read(decoder, depth + 1)?;
        entries.insert(key.to_owned(), value);
        previous = Some(key);
    }

    Ok(Value::Object(entries))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{MAX_DEPTH, decode, encode};
    use crate::payload::DetailsError;

    fn bytes(value: &Value) -> Vec<u8> {
        let mut out = Vec::new();
        encode(value, &mut out).unwrap();
        out
    }

    #[test]
    fn values_round_trip() {
        let value = json!({
            "command": "systemctl restart api",
            "exit": 0,
            "bytes": u64::MAX,
            "lines": [-1, 2, null, true, false],
            "nested": { "b": 1, "a": "x" }
        });

        assert_eq!(decode(&bytes(&value)), Ok(value));
    }

    #[test]
    fn key_order_does_not_change_the_bytes() {
        let one = serde_json::from_str::<Value>(r#"{"a":1,"b":2}"#).unwrap();
        let other = serde_json::from_str::<Value>(r#"{"b":2,"a":1}"#).unwrap();

        assert_eq!(bytes(&one), bytes(&other));
    }

    #[test]
    fn floats_are_refused() {
        let mut out = Vec::new();
        assert_eq!(
            encode(&json!({ "ratio": 0.5 }), &mut out),
            Err(DetailsError::Float)
        );
    }

    #[test]
    fn deep_nesting_is_refused_both_ways() {
        let mut value = json!(0);
        for _ in 0..=MAX_DEPTH {
            value = json!([value]);
        }

        let mut out = Vec::new();
        assert!(encode(&value, &mut out).is_err());

        let crafted: Vec<u8> = std::iter::repeat_n([6, 0, 0, 0, 1], MAX_DEPTH + 2)
            .flatten()
            .collect();
        assert!(decode(&crafted).is_err());
    }

    #[test]
    fn non_canonical_bytes_are_refused() {
        let unsorted = [7, 0, 0, 0, 2, 0, 0, 0, 1, b'b', 0, 0, 0, 0, 1, b'a', 0];
        assert!(decode(&unsorted).is_err());

        let small_unsigned = [4, 0, 0, 0, 0, 0, 0, 0, 1];
        assert!(decode(&small_unsigned).is_err());
    }
}
