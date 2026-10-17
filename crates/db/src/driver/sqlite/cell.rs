use rusqlite::Row;
use rusqlite::types::ValueRef;
use serde_json::{Value, json};

use crate::json_cell::JsonCell;

pub(super) struct Cell<'a>(ValueRef<'a>);

impl<'a> Cell<'a> {
    pub(super) fn new(value: ValueRef<'a>) -> Self {
        Self(value)
    }

    pub(super) fn row(row: &Row<'_>) -> Vec<Value> {
        (0..row.as_ref().column_count())
            .map(|index| {
                row.get_ref(index)
                    .map_or(Value::Null, |value| Cell::new(value).into())
            })
            .collect()
    }
}

impl From<Cell<'_>> for Value {
    fn from(cell: Cell<'_>) -> Self {
        match cell.0 {
            ValueRef::Null => Self::Null,
            ValueRef::Integer(number) => json!(number),
            ValueRef::Real(number) => JsonCell::float(number).into(),
            ValueRef::Text(bytes) => JsonCell::text_or_binary(bytes).into(),
            ValueRef::Blob(bytes) => JsonCell::binary(bytes).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::types::ValueRef;
    use serde_json::{Value, json};

    use super::Cell;

    fn decode(value: ValueRef<'_>) -> Value {
        Cell::new(value).into()
    }

    #[test]
    fn storage_classes_map_to_json() {
        assert_eq!(decode(ValueRef::Integer(7)), json!(7));
        assert_eq!(decode(ValueRef::Real(0.5)), json!(0.5));
        assert_eq!(decode(ValueRef::Text(b"hi")), json!("hi"));
        assert_eq!(decode(ValueRef::Blob(&[1, 2])), json!("\\x0102"));
        assert_eq!(decode(ValueRef::Null), json!(null));
    }
}
