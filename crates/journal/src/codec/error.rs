use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("malformed encoding: {0}")]
pub(crate) struct CodecError(pub(crate) &'static str);
