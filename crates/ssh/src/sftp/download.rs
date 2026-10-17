mod piece;

use std::sync::Arc;

use bytes::Bytes;
use russh_sftp::client::error::Error;

use piece::Piece;

use super::{InFlight, RemoteFile, Sftp, SftpResult};
use crate::SshError;

const OPERATION: &str = "read";

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

        Ok(Some(piece.into_bytes()))
    }

    async fn next_piece(&mut self) -> Result<Option<Piece>, SshError> {
        let answer = match self.held.take() {
            Some(answer) => answer,
            None => match self.requests.next().await {
                Some(answer) => answer,
                None => return Ok(None),
            },
        };

        match answer.or_end() {
            Ok(Some(piece)) if !piece.is_empty() => Ok(Some(piece)),
            Ok(_) => Ok(None),
            Err(error) => Err(self.failed(&error)),
        }
    }

    async fn complete_short(&mut self, piece: Piece) -> Result<Bytes, SshError> {
        let Some(following) = self.requests.next().await else {
            return self.fill_gap(piece).await;
        };

        match following.or_end() {
            Ok(Some(next)) if !next.is_empty() => self.held = Some(Ok(next)),
            Ok(_) => {
                self.finish();
                return Ok(piece.into_data());
            }
            Err(error) => self.held = Some(Err(error)),
        }

        self.fill_gap(piece).await
    }

    async fn fill_gap(&mut self, mut piece: Piece) -> Result<Bytes, SshError> {
        while piece.is_short() {
            let rest = Piece::request(&self.file, piece.next_offset(), piece.missing()).await;

            match rest.or_end() {
                Ok(Some(rest)) if !rest.is_empty() => piece.extend(&rest),
                Ok(_) => break,
                Err(error) => return Err(self.failed(&error)),
            }
        }

        if piece.is_short() {
            self.finish();
        }

        Ok(piece.into_bytes())
    }

    fn fill(&mut self) {
        while !self.requests.is_full() {
            let Some(length) = self.next_length() else {
                return;
            };

            let request = Piece::request(&self.file, self.requested, length);
            self.requests.push(request);
            self.requested += u64::from(length);
        }
    }

    fn next_length(&self) -> Option<u32> {
        let whole = Piece::largest();

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

    fn failed(&mut self, error: &Error) -> SshError {
        self.finish();

        SshError::sftp(OPERATION, self.file.path(), error)
    }
}
