use std::error::Error;

use fallible_iterator::FallibleIterator;
use postgres_protocol::types as wire;
use serde_json::{Value, json};
use tokio_postgres::Row;
use tokio_postgres::types::{FromSql, Kind, Type};

use super::numeric::Numeric;
use crate::calendar;
use crate::cells::{binary, float, text_or_binary};

const DAYS_FROM_1970_TO_2000: i64 = 10_957;
const MICROS_FROM_1970_TO_2000: i64 = DAYS_FROM_1970_TO_2000 * 86_400_000_000;
const JSONB_VERSION: u8 = 1;
const UTC: &str = "+00";
const IPV4_BITS: u8 = 32;
const IPV6_BITS: u8 = 128;

pub(super) struct Cell<'a> {
    kind: Type,
    bytes: Option<&'a [u8]>,
}

impl<'a> Cell<'a> {
    pub(super) fn new(kind: Type, bytes: Option<&'a [u8]>) -> Self {
        Self { kind, bytes }
    }

    pub(super) fn read_row(row: &Row) -> Vec<Value> {
        (0..row.len())
            .map(|index| {
                row.try_get::<_, Cell<'_>>(index)
                    .map_or(Value::Null, Cell::into_json)
            })
            .collect()
    }

    pub(super) fn into_json(self) -> Value {
        let Some(bytes) = self.bytes else {
            return Value::Null;
        };

        self.known(bytes).unwrap_or_else(|| text_or_binary(bytes))
    }

    fn known(&self, bytes: &[u8]) -> Option<Value> {
        if let Kind::Array(element) = self.kind.kind() {
            return array(element, bytes);
        }

        let value = match self.kind {
            Type::BOOL => Value::Bool(wire::bool_from_sql(bytes).ok()?),
            Type::INT2 => json!(wire::int2_from_sql(bytes).ok()?),
            Type::INT4 => json!(wire::int4_from_sql(bytes).ok()?),
            Type::INT8 => json!(wire::int8_from_sql(bytes).ok()?),
            Type::OID => json!(wire::oid_from_sql(bytes).ok()?),
            Type::FLOAT4 => float(f64::from(wire::float4_from_sql(bytes).ok()?)),
            Type::FLOAT8 => float(wire::float8_from_sql(bytes).ok()?),
            Type::NUMERIC => Value::String(Numeric::read(bytes)?.to_string()),
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
            Type::TIMESTAMPTZ => {
                Value::String(timestamp(wire::timestamp_from_sql(bytes).ok()?, UTC))
            }
            Type::INET | Type::CIDR => Value::String(inet(bytes)?),
            _ => return None,
        };

        Some(value)
    }
}

impl<'a> FromSql<'a> for Cell<'a> {
    fn from_sql(kind: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Self::new(kind.clone(), Some(raw)))
    }

    fn from_sql_null(kind: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Self::new(kind.clone(), None))
    }

    fn accepts(_kind: &Type) -> bool {
        true
    }
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
        .map(|value| Ok(Cell::new(element.clone(), value).into_json()))
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
    let full = if inet.addr().is_ipv4() {
        IPV4_BITS
    } else {
        IPV6_BITS
    };

    if inet.netmask() == full {
        return Some(inet.addr().to_string());
    }

    Some(format!("{}/{}", inet.addr(), inet.netmask()))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use tokio_postgres::types::{Kind, Type};

    use super::Cell;

    fn decode(kind: &Type, bytes: Option<&[u8]>) -> Value {
        Cell::new(kind.clone(), bytes).into_json()
    }

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
            Kind::Simple,
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
