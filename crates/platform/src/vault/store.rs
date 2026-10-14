use zeroize::Zeroizing;

use super::KeyPlace;
use crate::PlatformError;

pub trait Vault: Send + Sync {
    fn place(&self) -> KeyPlace;

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError>;

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError>;

    fn remove(&self, name: &str) -> Result<(), PlatformError>;
}
