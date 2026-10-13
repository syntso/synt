#![allow(dead_code, unused_imports)]

mod clock;
mod file_key_store;
mod flaky_key_store;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::Connection;
use serde_json::json;
use synt_journal::{
    ActionId, Actor, Details, EventKind, Journal, JournalBuilder, JournalError, JournalId,
    JournalReader, KeyStore, MemoryKeyStore, NewEvent, ResourceRef,
};
use tempfile::TempDir;

pub use clock::ManualClock;
pub use file_key_store::FileKeyStore;
pub use flaky_key_store::FlakyKeyStore;

pub struct Fixture {
    pub dir: TempDir,
    pub path: PathBuf,
    pub keys: Arc<MemoryKeyStore>,
    pub clock: Arc<ManualClock>,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.sqlite");

        Self {
            dir,
            path,
            keys: Arc::new(MemoryKeyStore::new().unwrap()),
            clock: Arc::new(ManualClock::starting_at(1_700_000_000_000)),
        }
    }

    pub fn keys(&self) -> Arc<dyn KeyStore> {
        self.keys.clone()
    }

    pub fn create(&self) -> Journal {
        self.builder().create(&self.path).unwrap()
    }

    pub fn open(&self, id: JournalId) -> Result<Journal, JournalError> {
        self.builder().open(&self.path, id)
    }

    pub fn reader(&self, id: JournalId) -> JournalReader {
        JournalReader::open(&self.path, id, self.keys()).unwrap()
    }

    pub fn with_events(&self, count: u64) -> JournalId {
        let mut journal = self.create();

        for n in 1..=count {
            journal.append(&exec(&format!("step {n}"))).unwrap();
        }

        journal.id()
    }

    pub fn raw(&self) -> Connection {
        raw(&self.path)
    }

    fn builder(&self) -> JournalBuilder {
        JournalBuilder::new(self.keys()).clock(self.clock.clone())
    }
}

pub fn raw(path: &Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch("DROP TRIGGER IF EXISTS events_are_append_only")
        .unwrap();

    connection
}

pub fn exec(command: &str) -> NewEvent {
    NewEvent::new(Actor::Person, EventKind::Requested, action("ssh.exec"))
        .resource(ResourceRef::new("@host/api-1").unwrap())
        .details(Details::from_serialize(&json!({ "command": command })).unwrap())
}

pub fn action(id: &str) -> ActionId {
    ActionId::new(id).unwrap()
}
