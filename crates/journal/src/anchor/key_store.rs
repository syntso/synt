use super::Anchors;
use crate::chain::{DeviceKey, JournalId};
use crate::error::KeyStoreError;

pub trait KeyStore: Send + Sync {
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError>;

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError>;

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError>;

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError>;
}
