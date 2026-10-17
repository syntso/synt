use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;
use tokio::task::JoinError;

use crate::Engine;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("cannot connect: {0}")]
    Io(#[from] io::Error),

    #[error("cannot connect: {endpoint}: {source}")]
    Unreachable {
        endpoint: String,
        #[source]
        source: io::Error,
    },

    #[error("cannot connect: cannot open {}: {source}", .path.display())]
    Unopened {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },

    #[error("cannot connect: {}", described(.0))]
    PostgresConnect(#[source] tokio_postgres::Error),

    #[error("cannot connect: {0}")]
    MysqlConnect(#[source] mysql_async::Error),

    #[error("tls: {0}")]
    Tls(#[from] rustls::Error),

    #[error("tls: {0}")]
    Handshake(#[source] io::Error),

    #[error("tls: tls was asked for but not set up")]
    TlsNotSetUp,

    #[error("tls: {0:?} cannot be checked against a certificate")]
    UncheckableHost(String),

    #[error(
        "tls: no trusted root certificates were found on this computer, so verify-full cannot \
         check the server"
    )]
    NoRootCertificates,

    #[error("the server refused: {message}")]
    Server {
        message: String,
        code: Option<String>,
    },

    #[error("{0} takes commands, not sql")]
    TakesCommands(Engine),

    #[error("{0} takes sql, not a command")]
    TakesSql(Engine),

    #[error("{0} has no tables; look at keys with SCAN")]
    NoTables(Engine),

    #[error("{0}")]
    InvalidInput(String),

    #[error("no answer within {} s", .0.as_secs())]
    Timeout(Duration),

    #[error("the connection broke: {}", described(.0))]
    PostgresLost(#[source] tokio_postgres::Error),

    #[error("the connection broke: {0}")]
    MysqlLost(#[source] mysql_async::Error),

    #[error("the connection broke: {0}")]
    RedisLost(#[source] redis::RedisError),

    #[error("the connection broke: {0}")]
    Interrupted(#[from] JoinError),

    #[error("the connection broke: the catalog answered with an unexpected row")]
    UnexpectedRow,
}

impl DbError {
    pub(crate) fn server(message: impl Into<String>, code: Option<String>) -> Self {
        Self::Server {
            message: message.into(),
            code,
        }
    }
}

fn described(error: &tokio_postgres::Error) -> String {
    match error.source() {
        Some(source) => format!("{error}: {source}"),
        None => error.to_string(),
    }
}
