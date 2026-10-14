use std::sync::atomic::{AtomicBool, Ordering};

use synt_journal::{Anchors, DeviceKey, JournalId, KeyStore, KeyStoreError, MemoryKeyStore};

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
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        self.inner.device_key()
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        self.inner.ensure_device_key()
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError> {
        self.inner.anchors(journal)
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError> {
        if self.failing.load(Ordering::SeqCst) {
            return Err(KeyStoreError::new("the key store is unavailable"));
        }

        self.inner.save_anchors(journal, anchors)
    }
}
