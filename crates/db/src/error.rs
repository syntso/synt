use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DbError {
    #[error("cannot connect: {0}")]
    Connect(String),

    #[error("tls: {0}")]
    Tls(String),

    #[error("the server refused: {message}")]
    Server {
        message: String,
        code: Option<String>,
    },

    #[error("{0}")]
    Unsupported(String),

    #[error("{0}")]
    InvalidInput(String),

    #[error("no answer within {} s", .0.as_secs())]
    Timeout(Duration),

    #[error("the connection broke: {0}")]
    Lost(String),
}

impl DbError {
    pub(crate) fn server(message: impl Into<String>, code: Option<String>) -> Self {
        Self::Server {
            message: message.into(),
            code,
        }
    }
}
