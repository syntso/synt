use mysql_async::consts::ColumnType;
use mysql_async::{Column, Value as Cell};
use serde_json::{Value, json};

use crate::cells::{binary, float, text_or_binary};

const BINARY_CHARACTER_SET: u16 = 63;

pub(super) fn decode(column: &Column, cell: Cell) -> Value {
    match cell {
        Cell::NULL => Value::Null,
        Cell::Int(number) => json!(number),
        Cell::UInt(number) => json!(number),
        Cell::Float(number) => float(f64::from(number)),
        Cell::Double(number) => float(number),
        Cell::Date(year, month, day, hour, minute, second, micros) => Value::String(date(
            column,
            [year, month.into(), day.into()],
            [hour, minute, second],
            micros,
        )),
        Cell::Time(negative, days, hours, minutes, seconds, micros) => {
            Value::String(time(negative, days, [hours, minutes, seconds], micros))
        }
        Cell::Bytes(bytes) => bytes_value(column, &bytes),
    }
}

fn bytes_value(column: &Column, bytes: &[u8]) -> Value {
    let text = std::str::from_utf8(bytes).ok();

    match (column.column_type(), text) {
        (kind, Some(text)) if is_integer(kind) => text
            .parse::<i64>()
            .map(|number| json!(number))
            .or_else(|_| text.parse::<u64>().map(|number| json!(number)))
            .unwrap_or_else(|_| json!(text)),
        (ColumnType::MYSQL_TYPE_FLOAT | ColumnType::MYSQL_TYPE_DOUBLE, Some(text)) => {
            text.parse::<f64>().map_or_else(|_| json!(text), float)
        }
        (ColumnType::MYSQL_TYPE_JSON, Some(text)) => {
            serde_json::from_str(text).unwrap_or_else(|_| json!(text))
        }
        _ if column.character_set() == BINARY_CHARACTER_SET && !is_textual(column) => binary(bytes),
        _ => text_or_binary(bytes),
    }
}

fn is_integer(kind: ColumnType) -> bool {
    matches!(
        kind,
        ColumnType::MYSQL_TYPE_TINY
            | ColumnType::MYSQL_TYPE_SHORT
            | ColumnType::MYSQL_TYPE_INT24
            | ColumnType::MYSQL_TYPE_LONG
            | ColumnType::MYSQL_TYPE_LONGLONG
            | ColumnType::MYSQL_TYPE_YEAR
    )
}

fn is_textual(column: &Column) -> bool {
    matches!(
        column.column_type(),
        ColumnType::MYSQL_TYPE_DECIMAL
            | ColumnType::MYSQL_TYPE_NEWDECIMAL
            | ColumnType::MYSQL_TYPE_DATE
            | ColumnType::MYSQL_TYPE_DATETIME
            | ColumnType::MYSQL_TYPE_TIMESTAMP
            | ColumnType::MYSQL_TYPE_TIME
    )
}

fn date(column: &Column, day: [u16; 3], clock: [u8; 3], micros: u32) -> String {
    let [year, month, day] = day;
    let calendar = format!("{year:04}-{month:02}-{day:02}");

    if column.column_type() == ColumnType::MYSQL_TYPE_DATE {
        return calendar;
    }

    format!(
        "{calendar} {}",
        clock_text(u32::from(clock[0]), clock[1], clock[2], micros)
    )
}

fn time(negative: bool, days: u32, clock: [u8; 3], micros: u32) -> String {
    let hours = days * 24 + u32::from(clock[0]);
    let sign = if negative { "-" } else { "" };

    format!("{sign}{}", clock_text(hours, clock[1], clock[2], micros))
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
    use mysql_async::Column;
    use mysql_async::Value as Cell;
    use mysql_async::consts::ColumnType;
    use serde_json::json;

    use super::decode;

    fn column(kind: ColumnType) -> Column {
        Column::new(kind)
    }

    #[test]
    fn text_protocol_integers_become_numbers() {
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_LONG),
                Cell::Bytes(b"42".to_vec())
            ),
            json!(42)
        );
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_LONGLONG),
                Cell::Bytes(b"18446744073709551615".to_vec())
            ),
            json!(18_446_744_073_709_551_615_u64)
        );
    }

    #[test]
    fn decimals_stay_exact_text() {
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_NEWDECIMAL),
                Cell::Bytes(b"12.50".to_vec())
            ),
            json!("12.50")
        );
    }

    #[test]
    fn json_columns_are_parsed() {
        assert_eq!(
            decode(
                &column(ColumnType::MYSQL_TYPE_JSON),
                Cell::Bytes(br#"{"a":1}"#.to_vec())
            ),
            json!({"a": 1})
        );
    }

    #[test]
    fn binary_protocol_dates_and_times_are_written_out() {
        let datetime = Cell::Date(2026, 10, 14, 9, 5, 0, 250_000);
        let date = Cell::Date(2026, 10, 14, 0, 0, 0, 0);
        let time = Cell::Time(true, 1, 2, 3, 4, 0);

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

        assert_eq!(decode(&blob, Cell::Bytes(b"ok".to_vec())), json!("\\x6f6b"));
        assert_eq!(decode(&text, Cell::Bytes(b"ok".to_vec())), json!("ok"));
    }

    #[test]
    fn null_is_null() {
        assert_eq!(
            decode(&column(ColumnType::MYSQL_TYPE_LONG), Cell::NULL),
            json!(null)
        );
    }
}
