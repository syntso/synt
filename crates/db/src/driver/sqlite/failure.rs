use crate::DbError;

pub(super) fn failure(error: &rusqlite::Error) -> DbError {
    match error {
        rusqlite::Error::SqliteFailure(code, message) => DbError::server(
            message.clone().unwrap_or_else(|| code.to_string()),
            Some(format!("{:?}", code.code)),
        ),
        rusqlite::Error::MultipleStatement => DbError::InvalidInput(
            "sqlite runs one statement per call; send the others separately".to_owned(),
        ),
        rusqlite::Error::InvalidParameterCount(given, expected) => DbError::InvalidInput(format!(
            "the statement takes {expected} parameter(s), {given} given"
        )),
        other => DbError::server(other.to_string(), None),
    }
}
