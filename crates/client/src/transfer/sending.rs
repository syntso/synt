use bytes::BytesMut;
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use synt_proto::Credit;
use tokio::io::{AsyncRead, AsyncReadExt};

use super::Transferred;
use crate::transport::Connection;
use crate::{ClientError, StreamEvent, StreamReceiver};

const CHUNK: u64 = 64 * 1024;

pub(crate) async fn send<R: AsyncRead + Unpin>(
    connection: &Connection,
    stream: &mut StreamReceiver,
    mut source: R,
    size: u64,
) -> Result<Transferred, ClientError> {
    let mut hasher = Sha256::new();
    let mut allowed = 0;
    let mut sent = 0;

    while sent < size {
        while sent >= allowed {
            allowed = allowed.max(next_credit(stream).await?);
        }

        let wanted = CHUNK.min(size - sent).min(allowed - sent);
        let chunk = read_chunk(&mut source, wanted).await?;

        hasher.update(&chunk);
        sent += wanted;

        connection
            .send_stream(stream.stream(), chunk.freeze())
            .await?;
    }

    Ok(Transferred {
        bytes: sent,
        sha256: HEXLOWER.encode(&hasher.finalize()),
    })
}

async fn next_credit(stream: &mut StreamReceiver) -> Result<u64, ClientError> {
    match stream.next().await {
        Some(StreamEvent::Data(data)) => Credit::decode(&data)
            .map(|credit| credit.0)
            .ok_or_else(|| ClientError::Transfer("the core sent a malformed credit".to_owned())),
        Some(StreamEvent::Ended(_)) | None => Err(ClientError::Disconnected),
    }
}

async fn read_chunk<R: AsyncRead + Unpin>(
    source: &mut R,
    wanted: u64,
) -> Result<BytesMut, ClientError> {
    let length = usize::try_from(wanted).unwrap_or(usize::MAX);
    let mut chunk = BytesMut::zeroed(length);

    source.read_exact(&mut chunk).await?;

    Ok(chunk)
}
