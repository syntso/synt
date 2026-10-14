use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use super::{Frame, StreamId};
use crate::error::FrameError;

const HEADER_LENGTH: usize = 5;
const STREAM_ID_LENGTH: usize = 4;
const READ_AHEAD: usize = 64 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct FrameCodec {
    limit: usize,
}

impl FrameCodec {
    pub const DEFAULT_LIMIT: usize = 16 * 1024 * 1024;

    pub fn new() -> Self {
        Self {
            limit: Self::DEFAULT_LIMIT,
        }
    }

    pub fn with_limit(limit: usize) -> Self {
        Self { limit }
    }

    pub fn limit(self) -> usize {
        self.limit
    }

    fn too_large(self, size: usize) -> FrameError {
        FrameError::TooLarge {
            size,
            limit: self.limit,
        }
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = FrameError;

    fn decode(&mut self, source: &mut BytesMut) -> Result<Option<Frame>, FrameError> {
        if source.len() < HEADER_LENGTH {
            return Ok(None);
        }

        let length = u32::from_be_bytes([source[0], source[1], source[2], source[3]]);
        let length = usize::try_from(length).unwrap_or(usize::MAX);

        if length > self.limit {
            return Err(self.too_large(length));
        }
        if source.len() < HEADER_LENGTH + length {
            let missing = HEADER_LENGTH + length - source.len();
            source.reserve(missing.min(READ_AHEAD));
            return Ok(None);
        }

        let kind = source[4];

        source.advance(HEADER_LENGTH);
        let body = source.split_to(length);

        match kind {
            Frame::MESSAGE => Ok(Some(Frame::Message(body.freeze()))),
            Frame::STREAM => stream_frame(body).map(Some),
            unknown => Err(FrameError::UnknownKind(unknown)),
        }
    }
}

impl Encoder<Frame> for FrameCodec {
    type Error = FrameError;

    fn encode(&mut self, frame: Frame, target: &mut BytesMut) -> Result<(), FrameError> {
        let (kind, prefix, body) = match &frame {
            Frame::Message(body) => (Frame::MESSAGE, None, body),
            Frame::Stream { stream, data } => (Frame::STREAM, Some(stream.0), data),
        };

        let length = body.len() + prefix.map_or(0, |_| STREAM_ID_LENGTH);

        if length > self.limit {
            return Err(self.too_large(length));
        }

        let encoded_length = u32::try_from(length).map_err(|_| self.too_large(length))?;

        target.reserve(HEADER_LENGTH + length);
        target.put_u32(encoded_length);
        target.put_u8(kind);

        if let Some(stream) = prefix {
            target.put_u32(stream);
        }
        target.put_slice(body);

        Ok(())
    }
}

fn stream_frame(mut body: BytesMut) -> Result<Frame, FrameError> {
    if body.len() < STREAM_ID_LENGTH {
        return Err(FrameError::Truncated);
    }

    let stream = StreamId(body.get_u32());

    Ok(Frame::Stream {
        stream,
        data: body.freeze(),
    })
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};
    use tokio_util::codec::{Decoder, Encoder};

    use super::FrameCodec;
    use crate::error::FrameError;
    use crate::wire::{Frame, StreamId};

    fn encode(frame: Frame) -> BytesMut {
        let mut buffer = BytesMut::new();
        FrameCodec::new().encode(frame, &mut buffer).unwrap();
        buffer
    }

    #[test]
    fn frames_round_trip() {
        let frames = [
            Frame::message(Bytes::from_static(b"{\"jsonrpc\":\"2.0\"}")),
            Frame::Stream {
                stream: StreamId(7),
                data: Bytes::from_static(b"\x1b[32mok"),
            },
            Frame::message(Bytes::new()),
        ];

        let mut buffer = BytesMut::new();
        for frame in frames.clone() {
            FrameCodec::new().encode(frame, &mut buffer).unwrap();
        }

        let mut codec = FrameCodec::new();
        for expected in frames {
            assert_eq!(codec.decode(&mut buffer).unwrap(), Some(expected));
        }
        assert!(buffer.is_empty());
    }

    #[test]
    fn waits_for_the_whole_frame() {
        let whole = encode(Frame::message(Bytes::from_static(b"hello")));
        let mut codec = FrameCodec::new();

        for cut in 0..whole.len() {
            let mut partial = BytesMut::from(&whole[..cut]);
            assert_eq!(codec.decode(&mut partial).unwrap(), None);
        }
    }

    #[test]
    fn refuses_frames_over_the_limit_before_reading_them() {
        let mut header = BytesMut::from(&[0xff, 0xff, 0xff, 0xff, 1][..]);
        let refused = FrameCodec::with_limit(1024).decode(&mut header);

        assert!(matches!(refused, Err(FrameError::TooLarge { .. })));
    }

    #[test]
    fn a_bare_header_does_not_reserve_the_whole_body() {
        let mut header = BytesMut::from(&[0x00, 0xff, 0xff, 0xff, 1][..]);
        let waiting = FrameCodec::new().decode(&mut header).unwrap();

        assert_eq!(waiting, None);
        assert!(header.capacity() < 1024 * 1024);
    }

    #[test]
    fn refuses_unknown_kinds() {
        let mut buffer = BytesMut::from(&[0, 0, 0, 0, 9][..]);
        assert!(matches!(
            FrameCodec::new().decode(&mut buffer),
            Err(FrameError::UnknownKind(9))
        ));
    }

    #[test]
    fn refuses_stream_frames_without_an_id() {
        let mut buffer = BytesMut::from(&[0, 0, 0, 2, 2, 0, 0][..]);
        assert!(matches!(
            FrameCodec::new().decode(&mut buffer),
            Err(FrameError::Truncated)
        ));
    }
}
