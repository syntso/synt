use redis::RedisConnectionInfo;

use crate::{DbError, Server};

impl TryFrom<&Server> for RedisConnectionInfo {
    type Error = DbError;

    fn try_from(server: &Server) -> Result<Self, DbError> {
        let mut info = Self::default().set_skip_set_lib_name();

        if let Some(database) = server.database() {
            info = info.set_db(number(database)?);
        }
        if let Some(user) = server.user() {
            info = info.set_username(user);
        }
        if let Some(password) = server.password() {
            info = info.set_password(password);
        }

        Ok(info)
    }
}

fn number(database: &str) -> Result<i64, DbError> {
    database.parse().map_err(|_| {
        DbError::InvalidInput(format!(
            "a redis database is a number such as 0, got {database:?}"
        ))
    })
}
