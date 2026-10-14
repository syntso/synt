use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use crate::error::JournalError;

pub(crate) struct WriterLock {
    _file: File,
}

impl WriterLock {
    pub(crate) fn acquire(journal: &Path) -> Result<Self, JournalError> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(Self::path(journal))?;

        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(JournalError::Locked),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    fn path(journal: &Path) -> PathBuf {
        let mut path = journal.as_os_str().to_owned();
        path.push(".lock");

        PathBuf::from(path)
    }
}
