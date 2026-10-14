use std::fmt::{self, Debug, Formatter};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use tokio::time;
use zeroize::Zeroizing;

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
        if let Place::Network { reached_at, .. } = &mut self.place {
            *reached_at = Some(address);
        }

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

    pub(crate) fn host(&self) -> Option<&str> {
        match &self.place {
            Place::Network { host, .. } => Some(host),
            Place::File(_) => None,
        }
    }

    pub(crate) fn dial(&self) -> Result<(String, u16), DbError> {
        match &self.place {
            Place::Network {
                reached_at: Some(address),
                ..
            } => Ok((address.ip().to_string(), address.port())),
            Place::Network { host, port, .. } => Ok((host.clone(), *port)),
            Place::File(_) => Err(DbError::InvalidInput(format!(
                "{} needs a host and a port, not a file",
                self.engine
            ))),
        }
    }

    pub(crate) fn is_tunneled(&self) -> bool {
        matches!(
            self.place,
            Place::Network {
                reached_at: Some(_),
                ..
            }
        )
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
    use crate::Engine;

    #[test]
    fn debug_output_never_shows_the_password() {
        let server = Server::network(Engine::Postgres, "db", 5432)
            .with_password(Some(Zeroizing::new("hunter2".to_owned())));

        let shown = format!("{server:?}");

        assert!(!shown.contains("hunter2"), "{shown}");
        assert!(shown.contains("<hidden>"));
    }

    #[test]
    fn a_tunnel_changes_where_to_dial_but_not_the_host_name() {
        let server = Server::network(Engine::Mysql, "orders-db.internal", 3306)
            .reached_at("127.0.0.1:40123".parse().expect("address"));

        assert_eq!(server.dial(), Ok(("127.0.0.1".to_owned(), 40123)));
        assert_eq!(server.host(), Some("orders-db.internal"));
    }
}
