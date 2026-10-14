use std::io;

use synt_proto::{ProtoError, RpcError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("connection: {0}")]
    Io(#[from] io::Error),

    #[error("frame: {0}")]
    Frame(#[from] ProtoError),

    #[error("request cannot be encoded: {0}")]
    Encode(#[from] serde_json::Error),

    #[error("core refused: {0}")]
    Rpc(#[from] RpcError),

    #[error("cannot find synt-core to start it: {0}")]
    CoreNotFound(#[source] io::Error),

    #[error("the core must be named by an absolute path, not {program}")]
    CoreNotAbsolute { program: String },

    #[error("cannot start the core {program}: {source}")]
    CoreStart {
        program: String,
        #[source]
        source: io::Error,
    },

    #[error("started the core {program}, but it did not answer within {seconds} seconds")]
    CoreSilent { program: String, seconds: u64 },

    #[error("the connection to the core is closed")]
    Disconnected,

    #[error("the core sent a malformed credit")]
    MalformedCredit,

    #[error("the core replied with something unexpected: {0}")]
    UnexpectedReply(#[source] serde_json::Error),
}
