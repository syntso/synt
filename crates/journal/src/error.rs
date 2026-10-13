use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use thiserror::Error;

use crate::anchor::KeyStoreError;
use crate::chain::JournalId;
use crate::model::Seq;
use crate::verify::Verification;

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal database: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("journal file: {0}")]
    Io(#[from] io::Error),

    #[error("key store: {0}")]
    KeyStore(#[from] KeyStoreError),

    #[error("system random source: {0}")]
    Entropy(#[from] getrandom::Error),

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

    #[error("event {seq} cannot be read: {reason}")]
    Corrupt { seq: Seq, reason: &'static str },

    #[error("journal was tampered with: {0}")]
    Tampered(Verification),

    #[error("the journal writer has stopped")]
    WriterClosed,

    #[error("commit failed: {0}")]
    Commit(Arc<JournalError>),
}
