use rusqlite::Error;

use crate::DbError;

impl From<Error> for DbError {
    fn from(error: Error) -> Self {
        match error {
            Error::SqliteFailure(code, message) => Self::server(
                message.unwrap_or_else(|| code.to_string()),
                Some(format!("{:?}", code.code)),
            ),
            Error::MultipleStatement => Self::InvalidInput(
                "sqlite runs one statement per call; send the others separately".to_owned(),
            ),
            Error::InvalidParameterCount(given, expected) => Self::InvalidInput(format!(
                "the statement takes {expected} parameter(s), {given} given"
            )),
            other => Self::server(other.to_string(), None),
        }
    }
}
