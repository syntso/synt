use std::error::Error as _;

use crate::DbError;

pub(super) fn failure(error: &tokio_postgres::Error) -> DbError {
    if let Some(server) = error.as_db_error() {
        let mut message = server.message().to_owned();

        if let Some(detail) = server.detail() {
            message.push_str(": ");
            message.push_str(detail);
        }

        return DbError::server(message, Some(server.code().code().to_owned()));
    }

    if error.is_closed() {
        return DbError::Lost(described(error));
    }

    DbError::Connect(described(error))
}

fn described(error: &tokio_postgres::Error) -> String {
    match error.source() {
        Some(source) => format!("{error}: {source}"),
        None => error.to_string(),
    }
}
