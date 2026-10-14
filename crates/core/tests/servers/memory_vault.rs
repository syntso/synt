use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use synt_platform::{KeyPlace, PlatformError, Vault};
use zeroize::Zeroizing;

#[derive(Default)]
pub struct MemoryVault {
    items: Mutex<HashMap<String, Vec<u8>>>,
}

impl Vault for MemoryVault {
    fn place(&self) -> KeyPlace {
        KeyPlace::Files("memory".into())
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        let items = self.items.lock().unwrap_or_else(PoisonError::into_inner);

        Ok(items.get(name).cloned().map(Zeroizing::new))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        items.insert(name.to_owned(), bytes.to_vec());

        Ok(())
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        let mut items = self.items.lock().unwrap_or_else(PoisonError::into_inner);
        items.remove(name);

        Ok(())
    }
}
