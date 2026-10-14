use mysql_async::{DriverError, Error};

use crate::DbError;

pub(super) fn failure(error: &Error) -> DbError {
    match error {
        Error::Server(server) => DbError::server(
            server.message.clone(),
            Some(format!("{} ({})", server.code, server.state)),
        ),
        Error::Driver(DriverError::ConnectionClosed) => DbError::Lost(error.to_string()),
        Error::Io(io) => DbError::Connect(io.to_string()),
        other => DbError::Connect(other.to_string()),
    }
}

pub(super) fn lacks_tls(error: &Error) -> bool {
    matches!(error, Error::Driver(DriverError::NoClientSslFlagFromServer))
}
