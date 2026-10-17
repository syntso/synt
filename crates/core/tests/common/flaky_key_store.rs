use std::sync::atomic::{AtomicBool, Ordering};

use synt_journal::{Anchors, DeviceKey, JournalError, JournalId, KeyStore, MemoryKeyStore};

pub struct FlakyKeyStore {
    inner: MemoryKeyStore,
    failing: AtomicBool,
}

impl FlakyKeyStore {
    pub fn new() -> Self {
        Self {
            inner: MemoryKeyStore::new().unwrap(),
            failing: AtomicBool::new(false),
        }
    }

    pub fn fail_saves(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }
}

impl KeyStore for FlakyKeyStore {
    fn device_key(&self) -> Result<DeviceKey, JournalError> {
        self.inner.device_key()
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, JournalError> {
        self.inner.ensure_device_key()
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, JournalError> {
        self.inner.anchors(journal)
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), JournalError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(JournalError::key_store("the key store is unavailable"));
        }

        self.inner.save_anchors(journal, anchors)
    }
}
