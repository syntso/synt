use bytes::Bytes;

use super::StreamId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Message(Bytes),
    Stream { stream: StreamId, data: Bytes },
}

impl Frame {
    pub(super) const MESSAGE: u8 = 1;
    pub(super) const STREAM: u8 = 2;

    pub fn message(json: impl Into<Bytes>) -> Self {
        Self::Message(json.into())
    }
}
