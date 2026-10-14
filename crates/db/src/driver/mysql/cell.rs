use mysql_async::consts::ColumnType;
use mysql_async::{Column, Row, Value as MysqlValue};
use serde_json::{Value, json};

use crate::cells::{binary, float, text_or_binary};

const BINARY_CHARACTER_SET: u16 = 63;
const HOURS_PER_DAY: u32 = 24;

pub(super) struct Cell<'a> {
    column: &'a Column,
    value: MysqlValue,
}

impl<'a> Cell<'a> {
    pub(super) fn new(column: &'a Column, value: MysqlValue) -> Self {
        Self { column, value }
    }

    pub(super) fn read_row(mut row: Row) -> Vec<Value> {
        let columns = row.columns();

        columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let value = row.take(index).unwrap_or(MysqlValue::NULL);
                Cell::new(column, value).into_json()
            })
            .collect()
    }

    pub(super) fn into_json(self) -> Value {
        match self.value {
            MysqlValue::NULL => Value::Null,
            MysqlValue::Int(number) => json!(number),
            MysqlValue::UInt(number) => json!(number),
            MysqlValue::Float(number) => float(f64::from(number)),
            MysqlValue::Double(number) => float(number),
            MysqlValue::Date(year, month, day, hour, minute, second, micros) => {
                let day = [year, month.into(), day.into()];
                Value::String(self.date(day, [hour, minute, second], micros))
            }
            MysqlValue::Time(negative, days, hours, minutes, seconds, micros) => {
                Value::String(time(negative, days, [hours, minutes, seconds], micros))
            }
            MysqlValue::Bytes(ref bytes) => self.bytes(bytes),
        }
    }

    fn bytes(&self, bytes: &[u8]) -> Value {
        let text = std::str::from_utf8(bytes).ok();

        match (self.kind(), text) {
            (_, Some(text)) if self.is_integer() => integer(text),
            (ColumnType::MYSQL_TYPE_FLOAT | ColumnType::MYSQL_TYPE_DOUBLE, Some(text)) => {
                text.parse::<f64>().map_or_else(|_| json!(text), float)
            }
            (ColumnType::MYSQL_TYPE_JSON, Some(text)) => {
                serde_json::from_str(text).unwrap_or_else(|_| json!(text))
            }
            _ if self.is_binary() => binary(bytes),
            _ => text_or_binary(bytes),
        }
    }

    fn date(&self, day: [u16; 3], clock: [u8; 3], micros: u32) -> String {
        let [year, month, day] = day;
        let calendar = format!("{year:04}-{month:02}-{day:02}");

        if self.kind() == ColumnType::MYSQL_TYPE_DATE {
            return calendar;
        }

        let [hours, minutes, seconds] = clock;

        format!(
            "{calendar} {}",
            clock_text(u32::from(hours), minutes, seconds, micros)
        )
    }

    fn kind(&self) -> ColumnType {
        self.column.column_type()
    }

    fn is_integer(&self) -> bool {
        matches!(
            self.kind(),
            ColumnType::MYSQL_TYPE_TINY
                | ColumnType::MYSQL_TYPE_SHORT
                | ColumnType::MYSQL_TYPE_INT24
                | ColumnType::MYSQL_TYPE_LONG
                | ColumnType::MYSQL_TYPE_LONGLONG
                | ColumnType::MYSQL_TYPE_YEAR
        )
    }

    fn is_binary(&self) -> bool {
        self.column.character_set() == BINARY_CHARACTER_SET && !self.is_textual()
    }

    fn is_textual(&self) -> bool {
        matches!(
            self.kind(),
            ColumnType::MYSQL_TYPE_DECIMAL
                | ColumnType::MYSQL_TYPE_NEWDECIMAL
                | ColumnType::MYSQL_TYPE_DATE
                | ColumnType::MYSQL_TYPE_DATETIME
                | ColumnType::MYSQL_TYPE_TIMESTAMP
                | ColumnType::MYSQL_TYPE_TIME
        )
    }
}

fn integer(text: &str) -> Value {
    text.parse::<i64>()
        .map(|number| json!(number))
        .or_else(|_| text.parse::<u64>().map(|number| json!(number)))
        .unwrap_or_else(|_| json!(text))
}

fn time(negative: bool, days: u32, clock: [u8; 3], micros: u32) -> String {
    let [hours, minutes, seconds] = clock;
    let hours = days * HOURS_PER_DAY + u32::from(hours);
    let sign = if negative { "-" } else { "" };

    format!("{sign}{}", clock_text(hours, minutes, seconds, micros))
}

fn clock_text(hours: u32, minutes: u8, seconds: u8, micros: u32) -> String {
    let whole = format!("{hours:02}:{minutes:02}:{seconds:02}");

    if micros == 0 {
        return whole;
    }

    let fraction = format!("{micros:06}");

    format!("{whole}.{}", fraction.trim_end_matches('0'))
}

#[cfg(test)]
mod tests {
    use mysql_async::consts::ColumnType;
    use mysql_async::{Column, Value as MysqlValue};
    use serde_json::{Value, json};

    use super::Cell;

    fn column(kind: ColumnType) -> Column {
        Column::new(kind)
    }

    fn decode(column: &Column, value: MysqlValue) -> Value {
        Cell::new(column, value).into_json()
    }

    #[test]
    fn text_protocol_integers_become_numbers() {
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_LONG),
                MysqlValue::Bytes(b"42".to_vec())
            ),
            json!(42)
        );
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_LONGLONG),
                MysqlValue::Bytes(b"18446744073709551615".to_vec())
            ),
            json!(18_446_744_073_709_551_615_u64)
        );
    }

    #[test]
    fn decimals_stay_exact_text() {
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_NEWDECIMAL),
                MysqlValue::Bytes(b"12.50".to_vec())
            ),
            json!("12.50")
        );
    }

    #[test]
    fn json_columns_are_parsed() {
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_JSON),
                MysqlValue::Bytes(br#"{"a":1}"#.to_vec())
            ),
            json!({"a": 1})
        );
    }

    #[test]
    fn binary_protocol_dates_and_times_are_written_out() {
        let datetime = MysqlValue::Date(2026, 10, 14, 9, 5, 0, 250_000);
        let date = MysqlValue::Date(2026, 10, 14, 0, 0, 0, 0);
        let time = MysqlValue::Time(true, 1, 2, 3, 4, 0);

        assert_eq!(
            decode(&column(ColumnType::MYSQL_TYPE_DATETIME), datetime),
            json!("2026-10-14 09:05:00.25")
        );
        assert_eq!(
            decode(&column(ColumnType::MYSQL_TYPE_DATE), date),
            json!("2026-10-14")
        );
        assert_eq!(
            decode(&column(ColumnType::MYSQL_TYPE_TIME), time),
            json!("-26:03:04")
        );
    }

    #[test]
    fn blobs_are_hex_and_text_is_text() {
        let blob = Column::new(ColumnType::MYSQL_TYPE_BLOB).with_character_set(63);
        let text = Column::new(ColumnType::MYSQL_TYPE_VAR_STRING).with_character_set(255);

        assert_eq!(
            decode(&blob, MysqlValue::Bytes(b"ok".to_vec())),
            json!("\\x6f6b")
        );
        assert_eq!(
            decode(&text, MysqlValue::Bytes(b"ok".to_vec())),
            json!("ok")
        );
    }

    #[test]
    fn null_is_null() {
        assert_eq!(
            decode(&column(ColumnType::MYSQL_TYPE_LONG), MysqlValue::NULL),
            json!(null)
        );
    }
}
