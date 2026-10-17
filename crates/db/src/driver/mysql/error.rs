use mysql_async::{DriverError, Error};

use crate::DbError;

impl From<Error> for DbError {
    fn from(error: Error) -> Self {
        match error {
            Error::Server(server) => Self::server(
                server.message,
                Some(format!("{} ({})", server.code, server.state)),
            ),
            Error::Driver(DriverError::ConnectionClosed) => Self::MysqlLost(error),
            other => Self::MysqlConnect(other),
        }
    }
}
