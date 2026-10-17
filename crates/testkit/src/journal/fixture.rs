use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;
use synt_journal::{
    Journal, JournalBuilder, JournalError, JournalId, JournalReader, KeyStore, MemoryKeyStore,
};
use tempfile::TempDir;

use super::{ManualClock, exec};
use crate::TestError;

pub struct Fixture {
    pub dir: TempDir,
    pub path: PathBuf,
    pub keys: Arc<MemoryKeyStore>,
    pub clock: Arc<ManualClock>,
}

impl Fixture {
    pub fn new() -> Result<Self, TestError> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("journal.sqlite");

        Ok(Self {
            dir,
            path,
            keys: Arc::new(MemoryKeyStore::new()?),
            clock: Arc::new(ManualClock::starting_at(1_700_000_000_000)),
        })
    }

    pub fn keys(&self) -> Arc<dyn KeyStore> {
        self.keys.clone()
    }

    pub fn create(&self) -> Result<Journal, JournalError> {
        self.builder().create(&self.path)
    }

    pub fn open(&self, id: JournalId) -> Result<Journal, JournalError> {
        self.builder().open(&self.path, id)
    }

    pub fn reader(&self, id: JournalId) -> Result<JournalReader, JournalError> {
        JournalReader::open(&self.path, id, self.keys())
    }

    pub fn with_events(&self, count: u64) -> Result<JournalId, TestError> {
        let mut journal = self.create()?;

        for n in 1..=count {
            journal.append(&exec(&format!("step {n}"))?)?;
        }

        Ok(journal.id())
    }

    pub fn raw(&self) -> Result<Connection, rusqlite::Error> {
        raw(&self.path)
    }

    fn builder(&self) -> JournalBuilder {
        JournalBuilder::new(self.keys()).clock(self.clock.clone())
    }
}

pub fn raw(path: &Path) -> Result<Connection, rusqlite::Error> {
    let connection = Connection::open(path)?;
    connection.execute_batch("DROP TRIGGER IF EXISTS events_are_append_only")?;

    Ok(connection)
}
