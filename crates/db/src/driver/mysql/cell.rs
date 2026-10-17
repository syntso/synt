use mysql_async::consts::ColumnType;
use mysql_async::{Column, Row, Value as MysqlValue};
use serde_json::{Value, json};

use crate::calendar::{Clock, Date};
use crate::json_cell::JsonCell;

const BINARY_CHARACTER_SET: u16 = 63;
const HOURS_PER_DAY: i64 = 24;

pub(super) struct Cell<'a> {
    column: &'a Column,
    value: MysqlValue,
}

impl<'a> Cell<'a> {
    pub(super) fn new(column: &'a Column, value: MysqlValue) -> Self {
        Self { column, value }
    }

    pub(super) fn row(mut row: Row) -> Vec<Value> {
        let columns = row.columns();

        columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let value = row.take(index).unwrap_or(MysqlValue::NULL);
                Value::from(Cell::new(column, value))
            })
            .collect()
    }

    fn bytes(&self, bytes: &[u8]) -> Value {
        let text = std::str::from_utf8(bytes).ok();

        match (self.kind(), text) {
            (_, Some(text)) if self.is_integer() => integer(text),
            (ColumnType::MYSQL_TYPE_FLOAT | ColumnType::MYSQL_TYPE_DOUBLE, Some(text)) => text
                .parse::<f64>()
                .map_or_else(|_| json!(text), |number| JsonCell::float(number).into()),
            (ColumnType::MYSQL_TYPE_JSON, Some(text)) => {
                serde_json::from_str(text).unwrap_or_else(|_| json!(text))
            }
            _ if self.is_binary() => JsonCell::binary(bytes).into(),
            _ => JsonCell::text_or_binary(bytes).into(),
        }
    }

    fn date(&self, date: Date, clock: Clock) -> String {
        if self.kind() == ColumnType::MYSQL_TYPE_DATE {
            return date.to_string();
        }

        format!("{date} {clock}")
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

impl From<Cell<'_>> for Value {
    fn from(cell: Cell<'_>) -> Self {
        match cell.value {
            MysqlValue::NULL => Self::Null,
            MysqlValue::Int(number) => json!(number),
            MysqlValue::UInt(number) => json!(number),
            MysqlValue::Float(number) => JsonCell::float(f64::from(number)).into(),
            MysqlValue::Double(number) => JsonCell::float(number).into(),
            MysqlValue::Date(year, month, day, hours, minutes, seconds, micros) => {
                let date = Date::new(year.into(), month.into(), day.into());
                let clock = Clock::new(hours.into(), minutes.into(), seconds.into(), micros.into());

                Self::String(cell.date(date, clock))
            }
            MysqlValue::Time(negative, days, hours, minutes, seconds, micros) => {
                let hours = i64::from(days) * HOURS_PER_DAY + i64::from(hours);
                let clock = Clock::new(hours, minutes.into(), seconds.into(), micros.into());
                let sign = if negative { "-" } else { "" };

                Self::String(format!("{sign}{clock}"))
            }
            MysqlValue::Bytes(ref bytes) => cell.bytes(bytes),
        }
    }
}

fn integer(text: &str) -> Value {
    text.parse::<i64>()
        .map(|number| json!(number))
        .or_else(|_| text.parse::<u64>().map(|number| json!(number)))
        .unwrap_or_else(|_| json!(text))
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
        Cell::new(column, value).into()
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
