mod memory;

use crate::anchors::Anchors;
use crate::chain::DeviceKey;
use crate::error::JournalError;
use crate::journal::JournalId;

pub use memory::MemoryKeyStore;

pub trait KeyStore: Send + Sync {
    fn device_key(&self) -> Result<DeviceKey, JournalError>;

    fn ensure_device_key(&self) -> Result<DeviceKey, JournalError>;

    fn anchors(&self, journal: JournalId) -> Result<Option<Anchors>, JournalError>;

    fn save_anchors(&self, journal: JournalId, anchors: &Anchors) -> Result<(), JournalError>;
}
