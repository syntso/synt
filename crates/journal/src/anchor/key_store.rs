use std::error::Error;

use thiserror::Error;

use super::Anchors;
use crate::chain::{DeviceKey, JournalId};

pub trait KeyStore: Send + Sync {
    fn device_key(&self) -> Result<DeviceKey, KeyStoreError>;

    fn ensure_device_key(&self) -> Result<DeviceKey, KeyStoreError>;

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, KeyStoreError>;

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), KeyStoreError>;
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct KeyStoreError(Box<dyn Error + Send + Sync>);

impl KeyStoreError {
    pub fn new(source: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self(source.into())
    }
}
