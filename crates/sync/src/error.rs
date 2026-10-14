use std::io;

use synt_actions::ActionError;
use synt_platform::PlatformError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("{0}")]
    Invalid(String),

    #[error("{0}")]
    Project(String),

    #[error("project {0} does not sync; run sync.enable first")]
    NotEnabled(String),

    #[error("this device does not hold the key of project {0} yet; a member has to share it")]
    NoKey(String),

    #[error("this device was removed from project {0}")]
    Removed(String),

    #[error("the sync provider failed: {0}")]
    Provider(String),

    #[error("a revision was refused: {0}")]
    Refused(String),

    #[error("cryptography failed: {0}")]
    Crypto(&'static str),

    #[error("the system random source failed: {0}")]
    Random(String),

    #[error("the key vault failed: {0}")]
    Vault(#[from] PlatformError),

    #[error("the sync database failed: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("the sync work stopped unexpectedly")]
    Stopped,
}

impl SyncError {
    pub(crate) fn invalid(reason: impl Into<String>) -> Self {
        Self::Invalid(reason.into())
    }

    pub(crate) fn refused(reason: impl Into<String>) -> Self {
        Self::Refused(reason.into())
    }

    pub(crate) fn provider(reason: impl Into<String>) -> Self {
        Self::Provider(reason.into())
    }
}

impl From<SyncError> for ActionError {
    fn from(error: SyncError) -> Self {
        match error {
            SyncError::Invalid(reason) => Self::InvalidParams(reason),
            other => Self::failed(other.to_string()),
        }
    }
}
