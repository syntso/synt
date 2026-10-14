use std::io;
use std::path::PathBuf;

use synt_actions::ActionError;
use synt_journal::{JournalError, JournalId};
use synt_platform::{KeyPlace, PlatformError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("data folder: {0}")]
    Io(#[from] io::Error),

    #[error("key storage: {0}")]
    Platform(#[from] PlatformError),

    #[error(transparent)]
    Journal(#[from] JournalError),

    #[error("{0}")]
    Usage(String),

    #[error("login start: {0}")]
    LoginStart(PlatformError),

    #[error("state database: {0}")]
    State(#[from] rusqlite::Error),

    #[error("the work stopped unexpectedly")]
    Stopped,

    #[error("ssh: {0}")]
    Ssh(#[from] synt_ssh::SshError),

    #[error("{0}")]
    Project(String),

    #[error("{0}")]
    Grant(String),

    #[error("{0}")]
    SshKey(String),

    #[error("{0}")]
    Password(String),

    #[error("system random source: {0}")]
    Random(String),

    #[error("key storage item {0} is damaged")]
    DamagedKey(String),

    #[error(
        "the device key is missing from {0}; the journal cannot be checked without it, \
         and the core will not make a new key for an existing journal"
    )]
    DeviceKeyMissing(KeyPlace),

    #[error(
        "journal missing: {place} remembers journal {id}, but {journal} is gone; \
         the core will not start a new journal in its place"
    )]
    JournalMissing {
        id: JournalId,
        journal: PathBuf,
        place: KeyPlace,
    },

    #[error(
        "{journal} exists but {place} remembers no journal for it, so it cannot be checked; \
         the core will not start a new journal over it"
    )]
    UnknownJournal { journal: PathBuf, place: KeyPlace },
}

impl From<CoreError> for ActionError {
    fn from(error: CoreError) -> Self {
        Self::Failed(error.to_string())
    }
}
