use std::sync::Arc;

use synt_journal::{Anchors, DeviceKey, JournalId, KeyStore, KeyStoreError};
use synt_platform::Vault;

use crate::CoreError;

const DEVICE_KEY: &str = "device-key";

pub(crate) struct VaultKeyStore {
    vault: Arc<dyn Vault>,
}

impl VaultKeyStore {
    pub(crate) fn new(vault: Arc<dyn Vault>) -> Self {
        Self { vault }
    }

    fn stored_device_key(&self) -> Result<Option<DeviceKey>, CoreError> {
        let Some(bytes) = self.vault.read(DEVICE_KEY)? else {
            return Ok(None);
        };

        let key = DeviceKey::from_slice(&bytes).ok_or_else(|| damaged(DEVICE_KEY))?;

        Ok(Some(key))
    }

    fn new_device_key(&self) -> Result<DeviceKey, CoreError> {
        let key = DeviceKey::generate().map_err(|error| CoreError::Random(error.to_string()))?;

        self.vault.write(DEVICE_KEY, key.expose())?;

        Ok(key)
    }

    fn stored_anchors(&self, journal: JournalId) -> Result<Option<Anchors>, CoreError> {
        let name = anchors_name(journal);

        let Some(bytes) = self.vault.read(&name)? else {
            return Ok(None);
        };

        let anchors = Anchors::from_bytes(&bytes).ok_or_else(|| damaged(&name))?;

        Ok(Some(anchors))
    }
}

impl KeyStore for VaultKeyStore {
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        match self.stored_device_key().map_err(KeyStoreError::new)? {
            Some(key) => Ok(key),
            None => Err(KeyStoreError::new(CoreError::DeviceKeyMissing(
                self.vault.place(),
            ))),
        }
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        match self.stored_device_key().map_err(KeyStoreError::new)? {
            Some(key) => Ok(key),
            None => self.new_device_key().map_err(KeyStoreError::new),
        }
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError> {
        self.stored_anchors(journal).map_err(KeyStoreError::new)
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError> {
        let bytes = anchors.to_bytes();

        self.vault
            .write(&anchors_name(journal), &bytes)
            .map_err(KeyStoreError::new)
    }
}

fn anchors_name(journal: JournalId) -> String {
    format!("anchors.{journal}")
}

fn damaged(name: &str) -> CoreError {
    CoreError::DamagedKey(name.to_owned())
}
