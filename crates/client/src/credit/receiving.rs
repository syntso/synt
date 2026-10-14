use sha2::{Digest, Sha256};
use synt_proto::Credit;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::transport::Connection;
use crate::{ClientError, StreamEvent, StreamReceiver, Transferred};

const WINDOW: u64 = 4 * 1024 * 1024;

pub(crate) struct Receiving<'a> {
    connection: &'a Connection,
    stream: &'a mut StreamReceiver,
    hasher: Sha256,
    granted: u64,
    received: u64,
}

impl<'a> Receiving<'a> {
    pub(crate) fn new(connection: &'a Connection, stream: &'a mut StreamReceiver) -> Self {
        Self {
            connection,
            stream,
            hasher: Sha256::new(),
            granted: Credit::OPENING.get(),
            received: 0,
        }
    }

    pub(crate) async fn write_to<W: AsyncWrite + Unpin>(
        mut self,
        mut sink: W,
    ) -> Result<Transferred, ClientError> {
        loop {
            let data = match self.stream.next().await {
                Some(StreamEvent::Data(data)) => data,
                Some(StreamEvent::Ended(_)) | None => return Err(ClientError::Disconnected),
            };

            if data.is_empty() {
                break;
            }

            self.hasher.update(&data);
            sink.write_all(&data).await?;
            self.received += u64::try_from(data.len()).unwrap_or(u64::MAX);

            self.grant_more().await?;
        }

        sink.flush().await?;

        Ok(Transferred::hashed(self.received, self.hasher))
    }

    async fn grant_more(&mut self) -> Result<(), ClientError> {
        if self.received + WINDOW / 2 < self.granted {
            return Ok(());
        }

        self.granted = self.received + WINDOW;
        let credit = Credit::new(self.granted).encode();

        self.connection
            .send_stream(self.stream.stream(), credit)
            .await
    }
}
