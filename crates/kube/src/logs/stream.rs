use std::fmt::{self, Debug, Formatter};
use std::pin::Pin;

use bytes::Bytes;
use futures_util::io::{AsyncBufRead, AsyncReadExt};

use crate::KubeError;

const CHUNK: usize = 32 * 1024;

pub struct LogStream {
    reader: Pin<Box<dyn AsyncBufRead + Send>>,
}

impl LogStream {
    pub(crate) fn new(reader: impl AsyncBufRead + Send + 'static) -> Self {
        Self {
            reader: Box::pin(reader),
        }
    }

    pub async fn next(&mut self) -> Result<Option<Bytes>, KubeError> {
        let mut buffer = vec![0; CHUNK];
        let read = self.reader.read(&mut buffer).await?;

        if read == 0 {
            return Ok(None);
        }

        buffer.truncate(read);
        Ok(Some(Bytes::from(buffer)))
    }
}

impl Debug for LogStream {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("LogStream").finish_non_exhaustive()
    }
}
