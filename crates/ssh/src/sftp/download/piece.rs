use std::future::Future;
use std::sync::Arc;

use bytes::Bytes;
use russh_sftp::client::error::Error;

use crate::sftp::RemoteFile;
use crate::sftp::in_flight::REQUEST_BYTES;

pub(super) struct Piece {
    offset: u64,
    length: u32,
    data: Vec<u8>,
}

impl Piece {
    pub(super) fn request(
        file: &RemoteFile,
        offset: u64,
        length: u32,
    ) -> impl Future<Output = Result<Self, Error>> + Send + 'static {
        let session = Arc::clone(file.session());
        let handle = file.handle().to_owned();

        async move {
            let data = session.read(handle, offset, length).await?.data;

            Ok(Self {
                offset,
                length,
                data,
            })
        }
    }

    pub(super) fn largest() -> u32 {
        u32::try_from(REQUEST_BYTES).unwrap_or(u32::MAX)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub(super) fn is_short(&self) -> bool {
        self.data.len() < self.wanted()
    }

    pub(super) fn next_offset(&self) -> u64 {
        self.offset + u64::try_from(self.data.len()).unwrap_or(u64::MAX)
    }

    pub(super) fn missing(&self) -> u32 {
        u32::try_from(self.wanted() - self.data.len()).unwrap_or(u32::MAX)
    }

    pub(super) fn extend(&mut self, rest: &Piece) {
        self.data.extend_from_slice(&rest.data);
    }

    pub(super) fn into_data(self) -> Bytes {
        Bytes::from(self.data)
    }

    pub(super) fn into_bytes(mut self) -> Bytes {
        self.data.truncate(self.wanted());

        Bytes::from(self.data)
    }

    fn wanted(&self) -> usize {
        usize::try_from(self.length).unwrap_or(usize::MAX)
    }
}
