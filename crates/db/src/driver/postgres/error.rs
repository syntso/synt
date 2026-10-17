use tokio_postgres::Error;

use crate::DbError;

impl From<Error> for DbError {
    fn from(error: Error) -> Self {
        if let Some(server) = error.as_db_error() {
            let mut message = server.message().to_owned();

            if let Some(detail) = server.detail() {
                message.push_str(": ");
                message.push_str(detail);
            }

            return Self::server(message, Some(server.code().code().to_owned()));
        }

        if error.is_closed() {
            return Self::PostgresLost(error);
        }

        Self::PostgresConnect(error)
    }
}
