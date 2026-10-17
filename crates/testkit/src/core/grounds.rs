use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use synt_actions::{ActionError, Registry};
use synt_core::{AlwaysGrants, GrantSeal, Ping, State};
use synt_journal::{Journal, JournalWriter, MemoryKeyStore};
use synt_platform::{Endpoint, Vault};
use tempfile::TempDir;

use super::{Aimed, Confused, Failing, Guarded, Panicking, Secret, Touch, Waiting};
use crate::journal::FlakyKeyStore;
use crate::{MemoryVault, TestError};

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
    aim: Arc<Mutex<String>>,
}

impl Grounds {
    pub fn new() -> Result<Self, TestError> {
        let directory = tempfile::tempdir()?;
        let keys = Arc::new(FlakyKeyStore::new(Arc::new(MemoryKeyStore::new()?)));

        let journal = Journal::create(directory.path().join(JOURNAL_FILE), keys.clone())?;
        let journal = JournalWriter::spawn(journal)?;

        Ok(Self {
            directory,
            vault: Arc::default(),
            keys,
            journal,
            touches: Arc::default(),
            runs: Arc::default(),
            waits: Arc::default(),
            aim: Arc::new(Mutex::new("10.0.0.1".to_owned())),
        })
    }

    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn endpoint(&self) -> Endpoint {
        let unique = self
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
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

    pub fn state(&self) -> Result<State, TestError> {
        Ok(State::open(&self.state_path())?)
    }

    pub fn always(&self) -> Result<AlwaysGrants, TestError> {
        let seal = GrantSeal::open(self.vault())?;

        Ok(AlwaysGrants::load(self.state()?, seal)?)
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

    pub fn aim_at(&self, place: &str) {
        let mut aim = self.aim.lock().unwrap_or_else(PoisonError::into_inner);
        place.clone_into(&mut aim);
    }

    pub fn registry(&self) -> Result<Registry, ActionError> {
        let mut registry = Registry::new();

        registry.register(Ping)?;
        registry.register(Touch::new(Arc::clone(&self.touches)))?;
        registry.register(Failing)?;
        registry.register(Panicking)?;
        registry.register(Guarded::new(Arc::clone(&self.runs)))?;
        registry.register(Waiting::new(Arc::clone(&self.waits)))?;
        registry.register(Secret)?;
        registry.register(Confused)?;
        registry.register(Aimed::new(Arc::clone(&self.aim)))?;

        Ok(registry)
    }
}
