use std::sync::Arc;

use redis::AsyncConnectionConfig;
use redis::aio::MultiplexedConnection;
use redis::{RedisConnectionInfo, Value as Reply};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_rustls::TlsConnector;

use super::reply::rows;
use crate::paging::Collector;

use crate::{DbError, Page, Rows, Server, TlsMode};

const VERSION_FIELD: &str = "redis_version:";

pub(crate) struct Redis {
    connection: MultiplexedConnection,
    driver: JoinHandle<()>,
}

impl Redis {
    pub(crate) async fn connect(server: &Server) -> Result<Self, DbError> {
        let limit = server.connect_limit();

        time::timeout(limit, Self::open(server))
            .await
            .map_err(|_| DbError::Timeout(limit))?
    }

    pub(crate) async fn ping(&mut self) -> Result<String, DbError> {
        let info: String = redis::cmd("INFO")
            .arg("server")
            .query_async(&mut self.connection)
            .await
            .map_err(|error| failure(&error))?;

        let version = info
            .lines()
            .find_map(|line| line.strip_prefix(VERSION_FIELD))
            .unwrap_or_default();

        Ok(version.trim().to_owned())
    }

    pub(crate) async fn command<S: AsRef<str>>(
        &mut self,
        words: &[S],
        page: Page,
    ) -> Result<Rows, DbError> {
        let Some((name, arguments)) = words.split_first() else {
            return Err(DbError::InvalidInput("the command is empty".to_owned()));
        };

        let mut command = redis::cmd(name.as_ref());
        for argument in arguments {
            command.arg(argument.as_ref());
        }

        let reply: Reply = command
            .query_async(&mut self.connection)
            .await
            .map_err(|error| failure(&error))?;

        let (columns, found) = rows(reply);
        let mut collector = Collector::new(page);

        for row in found {
            collector.offer(row);
        }

        Ok(collector.finish(columns, None))
    }

    async fn open(server: &Server) -> Result<Self, DbError> {
        let (host, port) = server
            .dial()
            .ok_or_else(|| DbError::InvalidInput("redis needs a host and a port".to_owned()))?;
        let info = connection_info(server)?;

        let socket = TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|error| DbError::Connect(format!("{host}:{port}: {error}")))?;
        socket
            .set_nodelay(true)
            .map_err(|error| DbError::Connect(error.to_string()))?;

        match server.tls() {
            TlsMode::Disable => Self::start(&info, socket).await,
            TlsMode::Prefer => Err(DbError::InvalidInput(
                "redis does not negotiate tls; choose disable, require or verify-full".to_owned(),
            )),
            TlsMode::Require | TlsMode::VerifyFull => {
                let secured = secure(server, socket).await?;
                Self::start(&info, secured).await
            }
        }
    }

    async fn start<C>(info: &RedisConnectionInfo, stream: C) -> Result<Self, DbError>
    where
        C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (connection, driver) =
            MultiplexedConnection::new_with_config(info, stream, AsyncConnectionConfig::new())
                .await
                .map_err(|error| failure(&error))?;

        Ok(Self {
            connection,
            driver: tokio::spawn(driver),
        })
    }
}

impl Drop for Redis {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

fn connection_info(server: &Server) -> Result<RedisConnectionInfo, DbError> {
    let mut info = RedisConnectionInfo::default().set_skip_set_lib_name();

    if let Some(database) = server.database() {
        let number: i64 = database.parse().map_err(|_| {
            DbError::InvalidInput(format!(
                "a redis database is a number such as 0, got {database:?}"
            ))
        })?;
        info = info.set_db(number);
    }

    if let Some(user) = server.user() {
        info = info.set_username(user);
    }
    if let Some(password) = server.password() {
        info = info.set_password(password);
    }

    Ok(info)
}

async fn secure(
    server: &Server,
    socket: TcpStream,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>, DbError> {
    let config = server
        .tls()
        .client_config()?
        .ok_or_else(|| DbError::Tls("tls was asked for but not set up".to_owned()))?;
    let host = server.host().unwrap_or_default().to_owned();
    let name = ServerName::try_from(host.clone())
        .map_err(|_| DbError::Tls(format!("{host:?} cannot be checked against a certificate")))?;

    TlsConnector::from(Arc::clone(&config))
        .connect(name, socket)
        .await
        .map_err(|error| DbError::Tls(error.to_string()))
}

fn failure(error: &redis::RedisError) -> DbError {
    if let Some(code) = error.code() {
        return DbError::server(
            error.detail().unwrap_or(code).to_owned(),
            Some(code.to_owned()),
        );
    }

    if error.is_io_error() || error.is_connection_dropped() {
        return DbError::Lost(error.to_string());
    }

    DbError::server(error.to_string(), None)
}
