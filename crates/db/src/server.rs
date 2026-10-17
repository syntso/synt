use std::fmt::{self, Debug, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::time;
use zeroize::Zeroizing;

use crate::endpoint::Endpoint;
use crate::{DbError, Engine, Place, TlsMode};

const CONNECT_LIMIT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Server {
    engine: Engine,
    place: Place,
    database: Option<String>,
    user: Option<String>,
    password: Option<Zeroizing<String>>,
    tls: TlsMode,
    connect_limit: Duration,
}

impl Server {
    pub fn network(engine: Engine, host: impl Into<String>, port: u16) -> Self {
        let place = Place::Network {
            host: host.into(),
            port,
            reached_at: None,
        };

        Self::at(engine, place)
    }

    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::at(Engine::Sqlite, Place::File(path.into()))
    }

    #[must_use]
    pub fn with_database(mut self, database: Option<String>) -> Self {
        self.database = database;
        self
    }

    #[must_use]
    pub fn with_user(mut self, user: Option<String>) -> Self {
        self.user = user;
        self
    }

    #[must_use]
    pub fn with_password(mut self, password: Option<Zeroizing<String>>) -> Self {
        self.password = password;
        self
    }

    #[must_use]
    pub fn with_tls(mut self, tls: TlsMode) -> Self {
        self.tls = tls;
        self
    }

    #[must_use]
    pub fn with_connect_limit(mut self, limit: Duration) -> Self {
        self.connect_limit = limit;
        self
    }

    #[must_use]
    pub fn reached_at(mut self, address: SocketAddr) -> Self {
        self.place.reached_at(address);
        self
    }

    pub fn engine(&self) -> Engine {
        self.engine
    }

    pub fn place(&self) -> &Place {
        &self.place
    }

    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    pub fn tls(&self) -> TlsMode {
        self.tls
    }

    pub fn connect_limit(&self) -> Duration {
        self.connect_limit
    }

    pub(crate) fn password(&self) -> Option<&str> {
        self.password.as_ref().map(|password| password.as_str())
    }

    pub(crate) fn dial(&self) -> Result<Endpoint, DbError> {
        self.place.dial().ok_or_else(|| {
            DbError::InvalidInput(format!(
                "{} needs a host and a port, not a file",
                self.engine
            ))
        })
    }

    pub(crate) async fn connect_in_time<T>(
        &self,
        connecting: impl Future<Output = Result<T, DbError>>,
    ) -> Result<T, DbError> {
        let limit = self.connect_limit;

        time::timeout(limit, connecting)
            .await
            .map_err(|_| DbError::Timeout(limit))?
    }

    fn at(engine: Engine, place: Place) -> Self {
        Self {
            engine,
            place,
            database: None,
            user: None,
            password: None,
            tls: TlsMode::Disable,
            connect_limit: CONNECT_LIMIT,
        }
    }
}

impl Debug for Server {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Server")
            .field("engine", &self.engine)
            .field("place", &self.place)
            .field("database", &self.database)
            .field("user", &self.user)
            .field("password", &self.password.as_ref().map(|_| "<hidden>"))
            .field("tls", &self.tls)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use zeroize::Zeroizing;

    use super::Server;
    use crate::{DbError, Engine};

    #[test]
    fn debug_output_never_shows_the_password() {
        let server = Server::network(Engine::Postgres, "db", 5432)
            .with_password(Some(Zeroizing::new("hunter2".to_owned())));

        let shown = format!("{server:?}");

        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.contains("<hidden>"));
    }

    #[test]
    fn a_file_cannot_be_dialed() {
        let server = Server::file("shop.sqlite");

        assert!(matches!(server.dial(), Err(DbError::InvalidInput(_))));
    }
}
