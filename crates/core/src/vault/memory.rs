use std::collections::HashMap;
use std::sync::Mutex;

use synt_platform::{KeyPlace, Lock, PlatformError, Vault};
use zeroize::Zeroizing;

#[derive(Default)]
pub(crate) struct MemoryVault {
    items: Mutex<HashMap<String, Vec<u8>>>,
}

impl Vault for MemoryVault {
    fn place(&self) -> KeyPlace {
        KeyPlace::Files("memory".into())
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        Ok(self.items.locked().get(name).cloned().map(Zeroizing::new))
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        self.items.locked().insert(name.to_owned(), bytes.to_vec());

        Ok(())
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        self.items.locked().remove(name);

        Ok(())
    }
}
