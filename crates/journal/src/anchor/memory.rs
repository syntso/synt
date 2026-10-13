use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use super::{Anchors, KeyStore, KeyStoreError};
use crate::chain::{DeviceKey, JournalId};

#[derive(Debug)]
pub struct MemoryKeyStore {
    device_key: DeviceKey,
    anchors: Mutex<HashMap<JournalId, Anchors>>,
}

impl MemoryKeyStore {
    pub fn new() -> Result<Self, getrandom::Error> {
        DeviceKey::generate().map(Self::with_key)
    }

    pub fn with_key(device_key: DeviceKey) -> Self {
        Self {
            device_key,
            anchors: Mutex::default(),
        }
    }

    pub fn forget(&self, journal: JournalId) {
        self.lock().remove(&journal);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<JournalId, Anchors>> {
        self.anchors.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl KeyStore for MemoryKeyStore {
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        Ok(self.device_key.clone())
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        self.device_key()
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError> {
        Ok(self.lock().get(&journal).copied())
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError> {
        self.lock().insert(journal, *anchors);
        Ok(())
    }
}
