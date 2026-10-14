use std::net::IpAddr;

use tokio_postgres::Config;
use tokio_postgres::config::SslMode;

use crate::{DbError, Server, TlsMode};

const APPLICATION: &str = "synt";

impl TryFrom<&Server> for Config {
    type Error = DbError;

    fn try_from(server: &Server) -> Result<Self, DbError> {
        let endpoint = server.dial()?;
        let host = server.place().host().unwrap_or(endpoint.host());

        let mut config = Config::new();
        config
            .host(host)
            .port(endpoint.port())
            .application_name(APPLICATION)
            .connect_timeout(server.connect_limit())
            .ssl_mode(server.tls().into());

        if let Ok(address) = endpoint.host().parse::<IpAddr>() {
            config.hostaddr(address);
        }
        if let Some(database) = server.database() {
            config.dbname(database);
        }
        if let Some(user) = server.user() {
            config.user(user);
        }
        if let Some(password) = server.password() {
            config.password(password);
        }

        Ok(config)
    }
}

impl From<TlsMode> for SslMode {
    fn from(tls: TlsMode) -> Self {
        match tls {
            TlsMode::Disable => Self::Disable,
            TlsMode::Prefer => Self::Prefer,
            TlsMode::Require | TlsMode::VerifyFull => Self::Require,
        }
    }
}
