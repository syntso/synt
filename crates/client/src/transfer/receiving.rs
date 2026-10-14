use sha2::{Digest, Sha256};
use synt_proto::Credit;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::Transferred;
use crate::transport::Connection;
use crate::{ClientError, StreamEvent, StreamReceiver};

const WINDOW: u64 = 4 * 1024 * 1024;

pub(crate) async fn receive<W: AsyncWrite + Unpin>(
    connection: &Connection,
    stream: &mut StreamReceiver,
    mut sink: W,
) -> Result<Transferred, ClientError> {
    let mut hasher = Sha256::new();
    let mut granted = Credit::OPENING.0;
    let mut received = 0;

    loop {
        let data = match stream.next().await {
            Some(StreamEvent::Data(data)) => data,
            Some(StreamEvent::Ended(_)) | None => return Err(ClientError::Disconnected),
        };

        if data.is_empty() {
            break;
        }

        hasher.update(&data);
        sink.write_all(&data).await?;
        received += u64::try_from(data.len()).unwrap_or(u64::MAX);

        if received + WINDOW / 2 >= granted {
            granted = received + WINDOW;
            let credit = Credit(granted).encode();
            connection.send_stream(stream.stream(), credit).await?;
        }
    }

    sink.flush().await?;

    Ok(Transferred::hashed(received, hasher))
}
