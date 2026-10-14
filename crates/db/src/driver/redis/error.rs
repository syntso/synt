use redis::RedisError;

use crate::DbError;

impl From<RedisError> for DbError {
    fn from(error: RedisError) -> Self {
        if let Some(code) = error.code() {
            return Self::server(
                error.detail().unwrap_or(code).to_owned(),
                Some(code.to_owned()),
            );
        }

        if error.is_io_error() || error.is_connection_dropped() {
            return Self::Lost(error.to_string());
        }

        Self::server(error.to_string(), None)
    }
}
