mod cell;
mod error;
mod options;
mod reply;

use std::sync::Arc;

use redis::aio::MultiplexedConnection;
use redis::{AsyncConnectionConfig, RedisConnectionInfo, Value as RedisValue};
use rustls::pki_types::ServerName;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use self::reply::Reply;
use crate::driver::{Answer, Driver};
use crate::{DbError, Engine, Page, Rows, Server, TlsMode};

const VERSION_FIELD: &str = "redis_version:";

pub(crate) struct Redis {
    connection: MultiplexedConnection,
    background: JoinHandle<()>,
}

impl Redis {
    pub(crate) async fn connect(server: &Server) -> Result<Self, DbError> {
        server.connect_in_time(Self::open(server)).await
    }

    async fn open(server: &Server) -> Result<Self, DbError> {
        let endpoint = server.dial()?;
        let info = RedisConnectionInfo::try_from(server)?;

        let socket = TcpStream::connect((endpoint.host(), endpoint.port()))
            .await
            .map_err(|error| DbError::Connect(format!("{endpoint}: {error}")))?;
        socket
            .set_nodelay(true)
            .map_err(|error| DbError::Connect(error.to_string()))?;

        match server.tls() {
            TlsMode::Disable => Self::start(&info, socket).await,
            TlsMode::Prefer => Err(DbError::InvalidInput(
                "redis does not negotiate tls; choose disable, require or verify-full".to_owned(),
            )),
            TlsMode::Require | TlsMode::VerifyFull => {
                let secured = Self::secure(server, socket).await?;
                Self::start(&info, secured).await
            }
        }
    }

    async fn secure(server: &Server, socket: TcpStream) -> Result<TlsStream<TcpStream>, DbError> {
        let config = server
            .tls()
            .client_config()?
            .ok_or_else(|| DbError::Tls("tls was asked for but not set up".to_owned()))?;
        let host = server.place().host().unwrap_or_default().to_owned();
        let name = ServerName::try_from(host.clone()).map_err(|_| {
            DbError::Tls(format!("{host:?} cannot be checked against a certificate"))
        })?;

        TlsConnector::from(Arc::clone(&config))
            .connect(name, socket)
            .await
            .map_err(|error| DbError::Tls(error.to_string()))
    }

    async fn start<C>(info: &RedisConnectionInfo, stream: C) -> Result<Self, DbError>
    where
        C: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (connection, background) =
            MultiplexedConnection::new_with_config(info, stream, AsyncConnectionConfig::new())
                .await?;

        Ok(Self {
            connection,
            background: tokio::spawn(background),
        })
    }

    async fn version(&mut self) -> Result<String, DbError> {
        let info: String = redis::cmd("INFO")
            .arg("server")
            .query_async(&mut self.connection)
            .await?;

        let version = info
            .lines()
            .find_map(|line| line.strip_prefix(VERSION_FIELD))
            .unwrap_or_default();

        Ok(version.trim().to_owned())
    }

    async fn run(&mut self, words: &[&str], page: Page) -> Result<Rows, DbError> {
        let Some((name, arguments)) = words.split_first() else {
            return Err(DbError::InvalidInput("the command is empty".to_owned()));
        };

        let mut command = redis::cmd(name);
        for argument in arguments {
            command.arg(*argument);
        }

        let reply: RedisValue = command.query_async(&mut self.connection).await?;

        Ok(Reply::new(reply).rows(page))
    }
}

impl Driver for Redis {
    fn engine(&self) -> Engine {
        Engine::Redis
    }

    fn ping(&mut self) -> Answer<'_, String> {
        Box::pin(self.version())
    }

    fn command<'a>(&'a mut self, words: &'a [&'a str], page: Page) -> Answer<'a, Rows> {
        Box::pin(self.run(words, page))
    }
}

impl Drop for Redis {
    fn drop(&mut self) {
        self.background.abort();
    }
}
