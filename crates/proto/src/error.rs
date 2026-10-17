use std::io;

use thiserror::Error;

use crate::Name;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("connection: {0}")]
    Io(#[from] io::Error),

    #[error("frame of {size} bytes exceeds the {limit} byte limit")]
    TooLarge { size: usize, limit: usize },

    #[error("unknown frame kind {0}")]
    UnknownKind(u8),

    #[error("frame is too short for its kind")]
    Truncated,

    #[error("name is empty")]
    EmptyName,

    #[error("name is {len} bytes, the limit is {}", Name::MAX_LEN)]
    LongName { len: usize },

    #[error("name contains a control character")]
    ControlCharacter,

    #[error("{text:?} must be 1 to {longest} bytes on one line")]
    NotOneLine { text: String, longest: usize },

    #[error("{0:?} is not an actor")]
    UnknownActor(String),
}
