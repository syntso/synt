use std::fmt;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, MutexGuard};

use synt_platform::{Lock, Vault};
use zeroize::Zeroizing;

use super::{VaultKind, VaultName};
use crate::CoreError;

pub(crate) struct VaultItems<K> {
    vault: Arc<dyn Vault>,
    changes: Arc<Mutex<()>>,
    kind: PhantomData<K>,
}

impl<K: VaultKind> VaultItems<K> {
    pub(crate) fn new(vault: Arc<dyn Vault>) -> Self {
        Self {
            vault,
            changes: Arc::default(),
            kind: PhantomData,
        }
    }

    pub(crate) fn change(&self) -> MutexGuard<'_, ()> {
        self.changes.locked()
    }

    pub(crate) fn read(
        &self,
        name: &VaultName<K>,
    ) -> Result<Option<Zeroizing<Vec<u8>>>, CoreError> {
        Ok(self.vault.read(&name.vault_item())?)
    }

    pub(crate) fn write(&self, name: &VaultName<K>, bytes: &[u8]) -> Result<(), CoreError> {
        Ok(self.vault.write(&name.vault_item(), bytes)?)
    }

    pub(crate) fn remove(&self, name: &VaultName<K>) -> Result<(), CoreError> {
        Ok(self.vault.remove(&name.vault_item())?)
    }
}

impl<K> Clone for VaultItems<K> {
    fn clone(&self) -> Self {
        Self {
            vault: Arc::clone(&self.vault),
            changes: Arc::clone(&self.changes),
            kind: PhantomData,
        }
    }
}

impl<K> fmt::Debug for VaultItems<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VaultItems")
            .field("place", &self.vault.place())
            .finish_non_exhaustive()
    }
}
