use std::sync::Arc;

use russh_sftp::client::error::Error;

use super::in_flight::REQUEST_BYTES;
use super::{InFlight, RemoteFile, Sftp, SftpResult};
use crate::SshError;

const OPERATION: &str = "write";

pub struct Upload {
    file: RemoteFile,
    requests: InFlight<()>,
    pending: Vec<u8>,
    offset: u64,
    sync: bool,
    _sftp: Arc<Sftp>,
}

impl Upload {
    pub(super) fn new(sftp: Arc<Sftp>, file: RemoteFile, offset: u64, sync: bool) -> Self {
        Self {
            file,
            requests: InFlight::new(),
            pending: Vec::with_capacity(REQUEST_BYTES),
            offset,
            sync,
            _sftp: sftp,
        }
    }

    pub async fn write(&mut self, mut data: &[u8]) -> Result<(), SshError> {
        while !data.is_empty() {
            let room = REQUEST_BYTES - self.pending.len();
            let (now, later) = data.split_at(room.min(data.len()));

            self.pending.extend_from_slice(now);
            data = later;

            if self.pending.len() == REQUEST_BYTES {
                self.send_pending().await?;
            }
        }

        Ok(())
    }

    pub async fn finish(mut self) -> Result<(), SshError> {
        if !self.pending.is_empty() {
            self.send_pending().await?;
        }

        while let Some(answer) = self.requests.next().await {
            answer.map_err(|error| self.failed(&error))?;
        }

        let Self { file, sync, .. } = self;
        let path = file.path().to_owned();

        file.sync_and_close(sync).await.during(OPERATION, &path)
    }

    async fn send_pending(&mut self) -> Result<(), SshError> {
        if self.requests.is_full()
            && let Some(answer) = self.requests.next().await
        {
            answer.map_err(|error| self.failed(&error))?;
        }

        let data = std::mem::replace(&mut self.pending, Vec::with_capacity(REQUEST_BYTES));
        let length = u64::try_from(data.len()).unwrap_or(u64::MAX);

        let session = Arc::clone(self.file.session());
        let handle = self.file.handle().to_owned();
        let offset = self.offset;

        self.requests
            .push(async move { session.write(handle, offset, data).await.map(|_| ()) });
        self.offset += length;

        Ok(())
    }

    fn failed(&mut self, error: &Error) -> SshError {
        self.requests.cancel();

        SshError::sftp(OPERATION, self.file.path(), error)
    }
}
