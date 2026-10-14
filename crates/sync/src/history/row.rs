use rusqlite::Row;
use rusqlite::types::Type;
use serde::de::DeserializeOwned;

use crate::SyncError;

pub(super) trait FromRow: Sized {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self>;
}

pub(super) trait TypedRow {
    fn parsed<T>(
        &self,
        column: &str,
        parse: impl Fn(&str) -> Result<T, SyncError>,
    ) -> rusqlite::Result<T>;

    fn optional<T>(
        &self,
        column: &str,
        parse: impl Fn(&str) -> Result<T, SyncError>,
    ) -> rusqlite::Result<Option<T>>;

    fn json<T: DeserializeOwned>(&self, column: &str) -> rusqlite::Result<T> {
        self.parsed(column, |text| {
            serde_json::from_str(text).map_err(|error| SyncError::invalid(error.to_string()))
        })
    }
}

impl TypedRow for Row<'_> {
    fn parsed<T>(
        &self,
        column: &str,
        parse: impl Fn(&str) -> Result<T, SyncError>,
    ) -> rusqlite::Result<T> {
        let text: String = self.get(column)?;

        parse(&text).map_err(|error| damaged(self, column, error))
    }

    fn optional<T>(
        &self,
        column: &str,
        parse: impl Fn(&str) -> Result<T, SyncError>,
    ) -> rusqlite::Result<Option<T>> {
        let text: Option<String> = self.get(column)?;

        text.map(|text| parse(&text).map_err(|error| damaged(self, column, error)))
            .transpose()
    }
}

fn damaged(row: &Row<'_>, column: &str, error: SyncError) -> rusqlite::Error {
    let index = row.as_ref().column_index(column).unwrap_or_default();

    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
}
