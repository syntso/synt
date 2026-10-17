use std::error::Error;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::oneshot;

use crate::journal::JournalId;
use crate::seq::Seq;
use crate::verify::Verification;

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal database: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("journal file: {0}")]
    Io(#[from] io::Error),

    #[error("key store: {0}")]
    KeyStore(#[source] Box<dyn Error + Send + Sync>),

    #[error("system random source: {0}")]
    Entropy(#[from] getrandom::Error),

    #[error("key derivation: {0}")]
    KeyDerivation(#[from] hkdf::InvalidLength),

    #[error("a journal already exists at {0}")]
    AlreadyExists(PathBuf),

    #[error("no journal at {0}")]
    NotFound(PathBuf),

    #[error("journal {0} is gone: its file was deleted or moved")]
    Missing(JournalId),

    #[error("expected journal {expected}, the file holds journal {found}")]
    Foreign {
        expected: JournalId,
        found: JournalId,
    },

    #[error("journal {0} has events but no anchors in the key store")]
    AnchorsMissing(JournalId),

    #[error("another writer holds the journal")]
    Locked,

    #[error("journal schema {found} is newer than this build understands ({supported})")]
    SchemaTooNew { found: i64, supported: i64 },

    #[error("not a journal: {0}")]
    NotAJournal(&'static str),

    #[error("malformed encoding: {0}")]
    Malformed(&'static str),

    #[error("event {seq} cannot be read: {reason}")]
    Corrupt { seq: Seq, reason: &'static str },

    #[error("journal was tampered with: {0}")]
    Tampered(Verification),

    #[error("details could not be serialized: {0}")]
    Unserializable(#[source] serde_json::Error),

    #[error("details contain a floating point number")]
    Float,

    #[error("details nest deeper than {limit} levels")]
    TooDeep { limit: usize },

    #[error("details take {size} bytes, the limit is {limit}")]
    TooLarge { size: usize, limit: usize },

    #[error("details contain what looks like private key material")]
    KeyMaterial,

    #[error("the journal writer has stopped")]
    WriterClosed,

    #[error("there are no events to append")]
    NothingToAppend,

    #[error("commit failed: {0}")]
    Commit(Arc<JournalError>),
}

impl JournalError {
    pub(crate) fn no_file(path: &Path, journal: JournalId, anchored: bool) -> Self {
        if anchored {
            Self::Missing(journal)
        } else {
            Self::NotFound(path.to_owned())
        }
    }

    pub fn key_store(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self::KeyStore(source.into())
    }

    pub(crate) fn at(self, seq: Seq) -> Self {
        match self {
            Self::Malformed(reason) => Self::Corrupt { seq, reason },
            other => other,
        }
    }
}

impl From<oneshot::error::RecvError> for JournalError {
    fn from(_: oneshot::error::RecvError) -> Self {
        Self::WriterClosed
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for JournalError {
    fn from(_: std::sync::mpsc::SendError<T>) -> Self {
        Self::WriterClosed
    }
}
