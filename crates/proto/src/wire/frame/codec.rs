use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio_util::codec::{Decoder, Encoder, FramedRead, FramedWrite};

use super::Frame;
use crate::ProtoError;

const HEADER_LENGTH: usize = 5;
const READ_AHEAD: usize = 64 * 1024;

pub type FrameReader<S> = FramedRead<ReadHalf<S>, FrameCodec>;
pub type FrameWriter<S> = FramedWrite<WriteHalf<S>, FrameCodec>;

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

    pub fn split<S>(stream: S, incoming_limit: usize) -> (FrameReader<S>, FrameWriter<S>)
    where
        S: AsyncRead + AsyncWrite,
    {
        let (read, write) = tokio::io::split(stream);

        let reader = FramedRead::new(read, Self::with_limit(incoming_limit));
        let writer = FramedWrite::new(write, Self::new());

        (reader, writer)
    }

    pub fn limit(self) -> usize {
        self.limit
    }

    fn check(self, size: usize) -> Result<u32, ProtoError> {
        let too_large = ProtoError::TooLarge {
            size,
            limit: self.limit,
        };

        if size > self.limit {
            return Err(too_large);
        }

        u32::try_from(size).map_err(|_| too_large)
    }
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for FrameCodec {
    type Item = Frame;
    type Error = ProtoError;

    fn decode(&mut self, source: &mut BytesMut) -> Result<Option<Frame>, ProtoError> {
        if source.len() < HEADER_LENGTH {
            return Ok(None);
        }

        let length = u32::from_be_bytes([source[0], source[1], source[2], source[3]]);
        let length = usize::try_from(length).unwrap_or(usize::MAX);
        self.check(length)?;

        if source.len() < HEADER_LENGTH + length {
            let missing = HEADER_LENGTH + length - source.len();
            source.reserve(missing.min(READ_AHEAD));
            return Ok(None);
        }

        let kind = source[4];
        source.advance(HEADER_LENGTH);
        let body = source.split_to(length);

        Frame::decode(kind, body).map(Some)
    }
}

impl Encoder<Frame> for FrameCodec {
    type Error = ProtoError;

    fn encode(&mut self, frame: Frame, target: &mut BytesMut) -> Result<(), ProtoError> {
        let length = frame.encoded_length();
        let encoded_length = self.check(length)?;

        target.reserve(HEADER_LENGTH + length);
        target.put_u32(encoded_length);
        target.put_u8(frame.kind());

        if let Some(stream) = frame.stream_id() {
            target.put_u32(stream.get());
        }
        target.put_slice(frame.body());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Bytes, BytesMut};
    use tokio_util::codec::{Decoder, Encoder};

    use super::FrameCodec;
    use crate::ProtoError;
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
                stream: StreamId::new(7),
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

        assert!(matches!(refused, Err(ProtoError::TooLarge { .. })));
    }

    #[test]
    fn refuses_to_send_frames_over_the_limit() {
        let frame = Frame::message(Bytes::from(vec![0; 2048]));
        let refused = FrameCodec::with_limit(1024).encode(frame, &mut BytesMut::new());

        assert!(matches!(refused, Err(ProtoError::TooLarge { .. })));
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
            Err(ProtoError::UnknownKind(9))
        ));
    }

    #[test]
    fn refuses_stream_frames_without_an_id() {
        let mut buffer = BytesMut::from(&[0, 0, 0, 2, 2, 0, 0][..]);
        assert!(matches!(
            FrameCodec::new().decode(&mut buffer),
            Err(ProtoError::Truncated)
        ));
    }
}
