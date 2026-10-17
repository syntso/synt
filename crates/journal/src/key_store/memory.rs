use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::KeyStore;
use crate::anchors::Anchors;
use crate::chain::DeviceKey;
use crate::error::JournalError;
use crate::journal::JournalId;

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

    fn lock(&self) -> MutexGuard<'_, HashMap<JournalId, Anchors>> {
        self.anchors.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl KeyStore for MemoryKeyStore {
    fn device_key(&self) -> Result<DeviceKey, JournalError> {
        Ok(self.device_key.clone())
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, JournalError> {
        self.device_key()
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, JournalError> {
        Ok(self.lock().get(&journal).copied())
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), JournalError> {
        self.lock().insert(journal, *anchors);
        Ok(())
    }
}
