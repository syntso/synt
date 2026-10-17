use crate::SyncError;
use crate::id::{Generation, ProjectId};
use crate::report::Access;
use crate::seal::Keyring;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Standing {
    Member(Keyring),
    Waiting,
    Removed,
}

impl Standing {
    pub(crate) fn access(&self) -> Access {
        match self {
            Self::Member(_) => Access::Member,
            Self::Waiting => Access::Waiting,
            Self::Removed => Access::Removed,
        }
    }

    pub(crate) fn generation(&self) -> Option<Generation> {
        match self {
            Self::Member(keyring) => Some(keyring.generation()),
            Self::Waiting | Self::Removed => None,
        }
    }

    pub(crate) fn into_keyring(self, project: &ProjectId) -> Result<Keyring, SyncError> {
        match self {
            Self::Member(keyring) => Ok(keyring),
            Self::Waiting => Err(SyncError::NoKey(project.clone())),
            Self::Removed => Err(SyncError::Removed(project.clone())),
        }
    }
}
