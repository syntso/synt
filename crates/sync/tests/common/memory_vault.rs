use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use synt_platform::{KeyPlace, PlatformError, Vault};
use zeroize::Zeroizing;

#[derive(Default)]
pub struct MemoryVault {
    items: Mutex<BTreeMap<String, Vec<u8>>>,
}

impl MemoryVault {
    pub fn names(&self) -> Vec<String> {
        self.lock().keys().cloned().collect()
    }

    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        self.items.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Vault for MemoryVault {
    fn place(&self) -> KeyPlace {
        KeyPlace::Files("memory".into())
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        Ok(self.lock().get(name).cloned().map(Zeroizing::new))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        self.lock().insert(name.to_owned(), bytes.to_vec());
        Ok(())
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        self.lock().remove(name);
        Ok(())
    }
}
