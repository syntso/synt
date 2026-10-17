use std::error::Error;

use rusqlite::Row;
use rusqlite::types::Type;
use serde::de::DeserializeOwned;

pub(super) trait FromRow: Sized {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Self>;
}

pub(super) trait TypedRow {
    fn parsed<T>(&self, column: &str) -> rusqlite::Result<T>
    where
        T: TryFrom<String>,
        T::Error: Error + Send + Sync + 'static;

    fn parsed_or_null<T>(&self, column: &str) -> rusqlite::Result<Option<T>>
    where
        T: TryFrom<String>,
        T::Error: Error + Send + Sync + 'static;

    fn json<T: DeserializeOwned>(&self, column: &str) -> rusqlite::Result<T>;
}

impl TypedRow for Row<'_> {
    fn parsed<T>(&self, column: &str) -> rusqlite::Result<T>
    where
        T: TryFrom<String>,
        T::Error: Error + Send + Sync + 'static,
    {
        let text: String = self.get(column)?;

        T::try_from(text).map_err(|error| damaged(self, column, error))
    }

    fn parsed_or_null<T>(&self, column: &str) -> rusqlite::Result<Option<T>>
    where
        T: TryFrom<String>,
        T::Error: Error + Send + Sync + 'static,
    {
        let text: Option<String> = self.get(column)?;

        text.map(T::try_from)
            .transpose()
            .map_err(|error| damaged(self, column, error))
    }

    fn json<T: DeserializeOwned>(&self, column: &str) -> rusqlite::Result<T> {
        let text: String = self.get(column)?;

        serde_json::from_str(&text).map_err(|error| damaged(self, column, error))
    }
}

fn damaged(
    row: &Row<'_>,
    column: &str,
    error: impl Error + Send + Sync + 'static,
) -> rusqlite::Error {
    let index = row.as_ref().column_index(column).unwrap_or_default();

    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
}
