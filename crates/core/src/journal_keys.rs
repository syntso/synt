mod anchor_bytes;

use anchor_bytes::AnchorBytes;

use std::sync::Arc;

use synt_journal::{Anchors, DeviceKey, JournalId, KeyStore, KeyStoreError};
use synt_platform::Vault;
use zeroize::Zeroizing;

use crate::CoreError;

const DEVICE_KEY: &str = "device-key";
const DEVICE_KEY_LENGTH: usize = 32;

pub(crate) struct JournalKeys {
    vault: Arc<dyn Vault>,
}

impl JournalKeys {
    pub(crate) fn new(vault: Arc<dyn Vault>) -> Self {
        Self { vault }
    }

    fn stored_device_key(&self) -> Result<Option<DeviceKey>, CoreError> {
        let Some(bytes) = self.vault.read(DEVICE_KEY)? else {
            return Ok(None);
        };

        let bytes: [u8; DEVICE_KEY_LENGTH] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| CoreError::DamagedKey(DEVICE_KEY.to_owned()))?;
        let bytes = Zeroizing::new(bytes);

        Ok(Some(DeviceKey::from_bytes(*bytes)))
    }

    fn new_device_key(&self) -> Result<DeviceKey, CoreError> {
        let key = DeviceKey::generate()?;

        self.vault.write(DEVICE_KEY, key.expose())?;

        Ok(key)
    }

    fn stored_anchors(&self, journal: JournalId) -> Result<Option<Anchors>, CoreError> {
        let item = Self::anchors_item(journal);

        let Some(bytes) = self.vault.read(&item)? else {
            return Ok(None);
        };

        let anchors = AnchorBytes::new(bytes.to_vec())
            .anchors()
            .ok_or(CoreError::DamagedKey(item))?;

        Ok(Some(anchors))
    }

    fn anchors_item(journal: JournalId) -> String {
        format!("anchors.{journal}")
    }
}

impl KeyStore for JournalKeys {
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        let key = self
            .stored_device_key()?
            .ok_or_else(|| CoreError::DeviceKeyMissing(self.vault.place()))?;

        Ok(key)
    }

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError> {
        match self.stored_device_key()? {
            Some(key) => Ok(key),
            None => Ok(self.new_device_key()?),
        }
    }

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError> {
        Ok(self.stored_anchors(journal)?)
    }

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError> {
        let bytes = AnchorBytes::from(anchors);

        self.vault
            .write(&Self::anchors_item(journal), bytes.as_slice())
            .map_err(CoreError::from)?;

        Ok(())
    }
}
