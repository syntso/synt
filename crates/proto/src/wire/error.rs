use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("connection: {0}")]
    Io(#[from] io::Error),

    #[error("frame of {size} bytes exceeds the {limit} byte limit")]
    TooLarge { size: usize, limit: usize },

    #[error("unknown frame kind {0}")]
    UnknownKind(u8),

    #[error("frame is too short for its kind")]
    Truncated,
}
