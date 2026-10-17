use std::future::Future;

use bytes::Bytes;
use synt_kube::{KubeError, LogStream, Watch};

use super::Chunk;

const NEWLINE: u8 = b'\n';

pub(crate) trait Feed: Send {
    fn next_chunk(&mut self) -> impl Future<Output = Result<Option<Chunk>, KubeError>> + Send;
}

impl Feed for LogStream {
    async fn next_chunk(&mut self) -> Result<Option<Chunk>, KubeError> {
        let Some(data) = self.next().await? else {
            return Ok(None);
        };

        let items = data.split(|byte| *byte == NEWLINE).count() - 1;

        Ok(Some(Chunk { data, items }))
    }
}

impl Feed for Watch {
    async fn next_chunk(&mut self) -> Result<Option<Chunk>, KubeError> {
        loop {
            let notice = self.next().await?;

            let Ok(mut line) = serde_json::to_vec(&notice) else {
                continue;
            };
            line.push(NEWLINE);

            return Ok(Some(Chunk {
                data: Bytes::from(line),
                items: 1,
            }));
        }
    }
}
