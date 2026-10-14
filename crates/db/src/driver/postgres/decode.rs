use fallible_iterator::FallibleIterator;
use postgres_protocol::types as wire;
use serde_json::{Value, json};
use tokio_postgres::types::{Kind, Type};

use super::numeric::numeric;
use crate::calendar;
use crate::cells::{binary, float, text_or_binary};

const DAYS_FROM_1970_TO_2000: i64 = 10_957;
const MICROS_FROM_1970_TO_2000: i64 = DAYS_FROM_1970_TO_2000 * 86_400_000_000;
const JSONB_VERSION: u8 = 1;

pub(super) fn decode(kind: &Type, bytes: Option<&[u8]>) -> Value {
    let Some(bytes) = bytes else {
        return Value::Null;
    };

    known(kind, bytes).unwrap_or_else(|| text_or_binary(bytes))
}

fn known(kind: &Type, bytes: &[u8]) -> Option<Value> {
    if let Kind::Array(element) = kind.kind() {
        return array(element, bytes);
    }

    let value = match *kind {
        Type::BOOL => Value::Bool(wire::bool_from_sql(bytes).ok()?),
        Type::INT2 => json!(wire::int2_from_sql(bytes).ok()?),
        Type::INT4 => json!(wire::int4_from_sql(bytes).ok()?),
        Type::INT8 => json!(wire::int8_from_sql(bytes).ok()?),
        Type::OID => json!(wire::oid_from_sql(bytes).ok()?),
        Type::FLOAT4 => float(f64::from(wire::float4_from_sql(bytes).ok()?)),
        Type::FLOAT8 => float(wire::float8_from_sql(bytes).ok()?),
        Type::NUMERIC => Value::String(numeric(bytes)?),
        Type::CHAR => {
            json!(char::from(wire::char_from_sql(bytes).ok()?.cast_unsigned()).to_string())
        }
        Type::BYTEA => binary(bytes),
        Type::JSON => serde_json::from_slice(bytes).ok()?,
        Type::JSONB => jsonb(bytes)?,
        Type::UUID => Value::String(uuid(wire::uuid_from_sql(bytes).ok()?)),
        Type::DATE => Value::String(date(wire::date_from_sql(bytes).ok()?)),
        Type::TIME => Value::String(calendar::time_of_day(wire::time_from_sql(bytes).ok()?)),
        Type::TIMESTAMP => Value::String(timestamp(wire::timestamp_from_sql(bytes).ok()?, "")),
        Type::TIMESTAMPTZ => Value::String(timestamp(wire::timestamp_from_sql(bytes).ok()?, "+00")),
        Type::INET | Type::CIDR => Value::String(inet(bytes)?),
        _ => return None,
    };

    Some(value)
}

fn array(element: &Type, bytes: &[u8]) -> Option<Value> {
    let array = wire::array_from_sql(bytes).ok()?;

    let lengths: Vec<usize> = array
        .dimensions()
        .map(|dimension| Ok(usize::try_from(dimension.len).unwrap_or(0)))
        .collect()
        .ok()?;

    let values: Vec<Value> = array
        .values()
        .map(|value| Ok(decode(element, value)))
        .collect()
        .ok()?;

    Some(nested(values, &lengths))
}

fn nested(values: Vec<Value>, lengths: &[usize]) -> Value {
    let Some((_, inner)) = lengths.split_first() else {
        return Value::Array(values);
    };

    if inner.is_empty() {
        return Value::Array(values);
    }

    let size: usize = inner.iter().product::<usize>().max(1);
    let mut values = values.into_iter();
    let mut groups = Vec::new();

    loop {
        let group: Vec<Value> = values.by_ref().take(size).collect();

        if group.is_empty() {
            break;
        }

        groups.push(nested(group, inner));
    }

    Value::Array(groups)
}

fn jsonb(bytes: &[u8]) -> Option<Value> {
    let (version, text) = bytes.split_first()?;

    if *version != JSONB_VERSION {
        return None;
    }

    serde_json::from_slice(text).ok()
}

fn uuid(bytes: [u8; 16]) -> String {
    let hex = data_encoding::HEXLOWER.encode(&bytes);

    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn date(days_since_2000: i32) -> String {
    match days_since_2000 {
        i32::MAX => "infinity".to_owned(),
        i32::MIN => "-infinity".to_owned(),
        days => calendar::date(i64::from(days) + DAYS_FROM_1970_TO_2000),
    }
}

fn timestamp(micros_since_2000: i64, zone: &str) -> String {
    match micros_since_2000 {
        i64::MAX => "infinity".to_owned(),
        i64::MIN => "-infinity".to_owned(),
        micros => format!(
            "{}{zone}",
            calendar::timestamp(micros.saturating_add(MICROS_FROM_1970_TO_2000))
        ),
    }
}

fn inet(bytes: &[u8]) -> Option<String> {
    let inet = wire::inet_from_sql(bytes).ok()?;
    let full = if inet.addr().is_ipv4() { 32 } else { 128 };

    if inet.netmask() == full {
        return Some(inet.addr().to_string());
    }

    Some(format!("{}/{}", inet.addr(), inet.netmask()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio_postgres::types::Type;

    use super::decode;

    #[test]
    fn null_is_null_whatever_the_type() {
        assert_eq!(decode(&Type::INT4, None), json!(null));
    }

    #[test]
    fn integers_and_booleans_keep_their_json_types() {
        assert_eq!(decode(&Type::INT4, Some(&42_i32.to_be_bytes())), json!(42));
        assert_eq!(
            decode(&Type::INT8, Some(&(-7_i64).to_be_bytes())),
            json!(-7)
        );
        assert_eq!(decode(&Type::BOOL, Some(&[1])), json!(true));
    }

    #[test]
    fn jsonb_drops_its_version_byte() {
        assert_eq!(
            decode(&Type::JSONB, Some(b"\x01{\"a\":[1,2]}")),
            json!({"a": [1, 2]})
        );
    }

    #[test]
    fn uuids_are_written_in_their_usual_groups() {
        let bytes: Vec<u8> = (0_u8..16).collect();

        assert_eq!(
            decode(&Type::UUID, Some(&bytes)),
            json!("00010203-0405-0607-0809-0a0b0c0d0e0f")
        );
    }

    #[test]
    fn dates_and_timestamps_count_from_2000() {
        assert_eq!(
            decode(&Type::DATE, Some(&0_i32.to_be_bytes())),
            json!("2000-01-01")
        );
        assert_eq!(
            decode(&Type::TIMESTAMPTZ, Some(&1_500_000_i64.to_be_bytes())),
            json!("2000-01-01 00:00:01.5+00")
        );
        assert_eq!(
            decode(&Type::TIMESTAMP, Some(&i64::MAX.to_be_bytes())),
            json!("infinity")
        );
    }

    #[test]
    fn an_unknown_type_falls_back_to_text_or_hex() {
        let citext = Type::new(
            "citext".to_owned(),
            99_999,
            tokio_postgres::types::Kind::Simple,
            "public".to_owned(),
        );

        assert_eq!(decode(&citext, Some(b"Ann")), json!("Ann"));
        assert_eq!(decode(&citext, Some(&[0xff])), json!("\\xff"));
    }

    #[test]
    fn a_one_dimensional_array_becomes_a_list() {
        let mut bytes = Vec::new();
        bytes.extend(1_i32.to_be_bytes());
        bytes.extend(0_i32.to_be_bytes());
        bytes.extend(23_u32.to_be_bytes());
        bytes.extend(2_i32.to_be_bytes());
        bytes.extend(1_i32.to_be_bytes());
        for value in [5_i32, 6] {
            bytes.extend(4_i32.to_be_bytes());
            bytes.extend(value.to_be_bytes());
        }

        assert_eq!(decode(&Type::INT4_ARRAY, Some(&bytes)), json!([5, 6]));
    }
}
