mod codec;

use bytes::{Buf, Bytes, BytesMut};

use super::StreamId;
use crate::ProtoError;

pub use codec::{FrameCodec, FrameReader, FrameWriter};

const STREAM_ID_LENGTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    Message(Bytes),
    Stream { stream: StreamId, data: Bytes },
}

impl Frame {
    const MESSAGE: u8 = 1;
    const STREAM: u8 = 2;

    pub fn message(json: impl Into<Bytes>) -> Self {
        Self::Message(json.into())
    }

    fn decode(kind: u8, body: BytesMut) -> Result<Self, ProtoError> {
        match kind {
            Self::MESSAGE => Ok(Self::Message(body.freeze())),
            Self::STREAM => Self::stream(body),
            unknown => Err(ProtoError::UnknownKind(unknown)),
        }
    }

    fn stream(mut body: BytesMut) -> Result<Self, ProtoError> {
        if body.len() < STREAM_ID_LENGTH {
            return Err(ProtoError::Truncated);
        }

        let stream = StreamId::new(body.get_u32());

        Ok(Self::Stream {
            stream,
            data: body.freeze(),
        })
    }

    fn kind(&self) -> u8 {
        match self {
            Self::Message(_) => Self::MESSAGE,
            Self::Stream { .. } => Self::STREAM,
        }
    }

    fn stream_id(&self) -> Option<StreamId> {
        match self {
            Self::Message(_) => None,
            Self::Stream { stream, .. } => Some(*stream),
        }
    }

    fn body(&self) -> &Bytes {
        match self {
            Self::Message(body) | Self::Stream { data: body, .. } => body,
        }
    }

    fn encoded_length(&self) -> usize {
        let prefix = self.stream_id().map_or(0, |_| STREAM_ID_LENGTH);

        self.body().len() + prefix
    }
}
