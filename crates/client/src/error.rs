use std::io;

use synt_proto::{FrameError, RpcError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("connection: {0}")]
    Io(#[from] io::Error),

    #[error("frame: {0}")]
    Frame(#[from] FrameError),

    #[error("request cannot be encoded: {0}")]
    Encode(#[from] serde_json::Error),

    #[error("core refused: {0}")]
    Rpc(#[from] RpcError),

    #[error("the connection to the core is closed")]
    Disconnected,

    #[error("transfer: {0}")]
    Transfer(String),

    #[error("the core replied with something unexpected: {0}")]
    UnexpectedReply(String),
}
