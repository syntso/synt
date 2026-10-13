use std::sync::Arc;

use bytes::Bytes;
use russh_sftp::client::error::Error;

use super::in_flight::{InFlight, REQUEST_BYTES};
use super::{RemoteFile, Sftp, failure, is_eof};
use crate::SshError;

const OPERATION: &str = "read";

struct Piece {
    offset: u64,
    length: u32,
    data: Vec<u8>,
}

impl Piece {
    fn wanted(&self) -> usize {
        usize::try_from(self.length).unwrap_or(usize::MAX)
    }

    fn is_short(&self) -> bool {
        self.data.len() < self.wanted()
    }
}

pub struct Download {
    file: RemoteFile,
    requests: InFlight<Piece>,
    held: Option<Result<Piece, Error>>,
    requested: u64,
    end: Option<u64>,
    ended: bool,
    _sftp: Arc<Sftp>,
}

impl Download {
    pub(super) fn new(sftp: Arc<Sftp>, file: RemoteFile, offset: u64, length: Option<u64>) -> Self {
        Self {
            file,
            requests: InFlight::new(),
            held: None,
            requested: offset,
            end: length.map(|length| offset.saturating_add(length)),
            ended: false,
            _sftp: sftp,
        }
    }

    pub async fn next(&mut self) -> Result<Option<Bytes>, SshError> {
        if self.ended {
            return Ok(None);
        }

        self.fill();

        let Some(piece) = self.next_piece().await? else {
            self.finish();
            return Ok(None);
        };

        if piece.is_short() {
            return self.complete_short(piece).await.map(Some);
        }

        Ok(Some(whole(piece)))
    }

    async fn next_piece(&mut self) -> Result<Option<Piece>, SshError> {
        let answer = match self.held.take() {
            Some(answer) => answer,
            None => match self.requests.next().await {
                Some(answer) => answer,
                None => return Ok(None),
            },
        };

        match answer {
            Ok(piece) if piece.data.is_empty() => Ok(None),
            Ok(piece) => Ok(Some(piece)),
            Err(error) if is_eof(&error) => Ok(None),
            Err(error) => Err(self.failed(error)),
        }
    }

    async fn complete_short(&mut self, piece: Piece) -> Result<Bytes, SshError> {
        let Some(following) = self.requests.next().await else {
            return self.fill_gap(piece).await;
        };

        match following {
            Ok(next) if next.data.is_empty() => self.finish(),
            Err(error) if is_eof(&error) => self.finish(),
            other => {
                self.held = Some(other);
                return self.fill_gap(piece).await;
            }
        }

        Ok(Bytes::from(piece.data))
    }

    async fn fill_gap(&mut self, mut piece: Piece) -> Result<Bytes, SshError> {
        while piece.is_short() {
            let got = u64::try_from(piece.data.len()).unwrap_or(u64::MAX);
            let missing = u32::try_from(piece.wanted() - piece.data.len()).unwrap_or(u32::MAX);

            match read(&self.file, piece.offset + got, missing).await {
                Ok(rest) if !rest.data.is_empty() => piece.data.extend_from_slice(&rest.data),
                Ok(_) => break,
                Err(error) if is_eof(&error) => break,
                Err(error) => return Err(self.failed(error)),
            }
        }

        if piece.is_short() {
            self.finish();
        }

        Ok(whole(piece))
    }

    fn fill(&mut self) {
        while !self.requests.is_full() {
            let Some(length) = self.next_length() else {
                return;
            };

            let request = read(&self.file, self.requested, length);
            self.requests.push(request);
            self.requested += u64::from(length);
        }
    }

    fn next_length(&self) -> Option<u32> {
        let whole = u32::try_from(REQUEST_BYTES).unwrap_or(u32::MAX);

        let Some(end) = self.end else {
            return Some(whole);
        };

        let left = end.saturating_sub(self.requested);

        if left == 0 {
            return None;
        }

        Some(u32::try_from(left).map_or(whole, |left| left.min(whole)))
    }

    fn finish(&mut self) {
        self.ended = true;
        self.held = None;
        self.requests.cancel();
    }

    fn failed(&mut self, error: Error) -> SshError {
        self.finish();
        failure(OPERATION, self.file.path())(error)
    }
}

fn whole(mut piece: Piece) -> Bytes {
    piece.data.truncate(piece.wanted());

    Bytes::from(piece.data)
}

fn read(
    file: &RemoteFile,
    offset: u64,
    length: u32,
) -> impl Future<Output = Result<Piece, Error>> + Send + 'static {
    let session = Arc::clone(file.session());
    let handle = file.handle().to_owned();

    async move {
        let data = session.read(handle, offset, length).await?.data;

        Ok(Piece {
            offset,
            length,
            data,
        })
    }
}
