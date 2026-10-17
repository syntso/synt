use rusqlite::Row;
use rusqlite::types::{FromSqlError, Type, ValueRef};

pub(super) struct Cells<'a, 'r> {
    row: &'a Row<'r>,
}

impl<'a, 'r> Cells<'a, 'r> {
    pub(super) fn new(row: &'a Row<'r>) -> Self {
        Self { row }
    }

    pub(super) fn blob(&self, index: usize) -> rusqlite::Result<&'a [u8]> {
        let value = self.row.get_ref(index)?;

        value
            .as_blob()
            .map_err(|error| Self::conversion(index, Type::Blob, error))
    }

    pub(super) fn text(&self, index: usize) -> rusqlite::Result<&'a str> {
        let value = self.row.get_ref(index)?;

        value
            .as_str()
            .map_err(|error| Self::conversion(index, Type::Text, error))
    }

    pub(super) fn optional_text(&self, index: usize) -> rusqlite::Result<Option<&'a str>> {
        let value: ValueRef<'a> = self.row.get_ref(index)?;

        value
            .as_str_or_null()
            .map_err(|error| Self::conversion(index, Type::Text, error))
    }

    fn conversion(index: usize, expected: Type, error: FromSqlError) -> rusqlite::Error {
        rusqlite::Error::FromSqlConversionFailure(index, expected, Box::new(error))
    }
}
