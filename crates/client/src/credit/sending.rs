use bytes::BytesMut;
use sha2::{Digest, Sha256};
use synt_proto::Credit;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::transport::Connection;
use crate::{ClientError, StreamEvent, StreamReceiver, Transferred};

const CHUNK: u64 = 64 * 1024;

pub(crate) struct Sending<'a> {
    connection: &'a Connection,
    stream: &'a mut StreamReceiver,
    hasher: Sha256,
    allowed: u64,
    sent: u64,
}

impl<'a> Sending<'a> {
    pub(crate) fn new(connection: &'a Connection, stream: &'a mut StreamReceiver) -> Self {
        Self {
            connection,
            stream,
            hasher: Sha256::new(),
            allowed: 0,
            sent: 0,
        }
    }

    pub(crate) async fn read_from<R: AsyncRead + Unpin>(
        mut self,
        mut source: R,
        size: u64,
    ) -> Result<Transferred, ClientError> {
        while self.sent < size {
            self.wait_for_credit().await?;

            let wanted = CHUNK.min(size - self.sent).min(self.allowed - self.sent);
            let chunk = Self::read_chunk(&mut source, wanted).await?;

            self.hasher.update(&chunk);
            self.sent += wanted;

            self.connection
                .send_stream(self.stream.stream(), chunk.freeze())
                .await?;
        }

        Ok(Transferred::hashed(self.sent, self.hasher))
    }

    async fn wait_for_credit(&mut self) -> Result<(), ClientError> {
        while self.sent >= self.allowed {
            let credit = self.next_credit().await?;
            self.allowed = self.allowed.max(credit.get());
        }

        Ok(())
    }

    async fn next_credit(&mut self) -> Result<Credit, ClientError> {
        match self.stream.next().await {
            Some(StreamEvent::Data(data)) => {
                Credit::decode(&data).ok_or(ClientError::MalformedCredit)
            }
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
}
