use tokio::io::{AsyncRead, AsyncWrite, ReadHalf, WriteHalf};
use tokio_util::codec::{FramedRead, FramedWrite};

use super::FrameCodec;

pub type FrameReader<S> = FramedRead<ReadHalf<S>, FrameCodec>;
pub type FrameWriter<S> = FramedWrite<WriteHalf<S>, FrameCodec>;

pub fn split_framed<S>(stream: S, incoming_limit: usize) -> (FrameReader<S>, FrameWriter<S>)
where
    S: AsyncRead + AsyncWrite,
{
    let (read, write) = tokio::io::split(stream);

    let reader = FramedRead::new(read, FrameCodec::with_limit(incoming_limit));
    let writer = FramedWrite::new(write, FrameCodec::new());

    (reader, writer)
}
