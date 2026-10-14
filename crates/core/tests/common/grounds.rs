use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use synt_actions::Registry;
use synt_core::{AlwaysGrants, GrantSeal, Ping, State};
use synt_journal::{Journal, JournalWriter};
use synt_platform::{Endpoint, Vault};
use tempfile::TempDir;

use super::{
    Confused, Failing, FlakyKeyStore, Guarded, MemoryVault, Panicking, Secret, Touch, Waiting,
};

const STATE_FILE: &str = "state.sqlite";
const JOURNAL_FILE: &str = "journal.sqlite";

pub struct Grounds {
    directory: TempDir,
    vault: Arc<MemoryVault>,
    keys: Arc<FlakyKeyStore>,
    journal: JournalWriter,
    touches: Arc<AtomicUsize>,
    runs: Arc<AtomicUsize>,
    waits: Arc<AtomicUsize>,
}

impl Grounds {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let keys = Arc::new(FlakyKeyStore::new());

        let journal = Journal::create(directory.path().join(JOURNAL_FILE), keys.clone()).unwrap();
        let journal = JournalWriter::spawn(journal).unwrap();

        Self {
            directory,
            vault: Arc::default(),
            keys,
            journal,
            touches: Arc::default(),
            runs: Arc::default(),
            waits: Arc::default(),
        }
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn endpoint(&self) -> Endpoint {
        let unique = self.path().file_name().unwrap().to_string_lossy();
        Endpoint::named(self.path(), &format!("synt-test-{unique}"))
    }

    pub fn journal(&self) -> JournalWriter {
        self.journal.clone()
    }

    pub fn fail_journal(&self, failing: bool) {
        self.keys.fail_saves(failing);
    }

    pub fn vault(&self) -> Arc<dyn Vault> {
        self.vault.clone()
    }

    pub fn vault_items(&self) -> Vec<String> {
        self.vault.names()
    }

    pub fn state_path(&self) -> PathBuf {
        self.path().join(STATE_FILE)
    }

    pub fn state(&self) -> State {
        State::open(&self.state_path()).unwrap()
    }

    pub fn always(&self) -> AlwaysGrants {
        let seal = GrantSeal::open(self.vault()).unwrap();

        AlwaysGrants::load(self.state(), seal).unwrap()
    }

    pub fn touches(&self) -> usize {
        self.touches.load(Ordering::SeqCst)
    }

    pub fn runs(&self) -> usize {
        self.runs.load(Ordering::SeqCst)
    }

    pub fn waits(&self) -> usize {
        self.waits.load(Ordering::SeqCst)
    }

    pub fn registry(&self) -> Registry {
        let mut registry = Registry::new();

        registry.register(Ping).unwrap();
        registry
            .register(Touch::new(Arc::clone(&self.touches)))
            .unwrap();
        registry.register(Failing).unwrap();
        registry.register(Panicking).unwrap();
        registry
            .register(Guarded::new(Arc::clone(&self.runs)))
            .unwrap();
        registry
            .register(Waiting::new(Arc::clone(&self.waits)))
            .unwrap();
        registry.register(Secret).unwrap();
        registry.register(Confused).unwrap();

        registry
    }
}
