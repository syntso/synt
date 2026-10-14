use std::collections::HashMap;
use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::{Entry, Error};
use windows_native_keyring_store::Store;
use zeroize::Zeroizing;

use super::{KeyPlace, Vault};
use crate::PlatformError;

const PERSISTENCE: &str = "persistence";
const THIS_MACHINE: &str = "Local";

pub(super) struct Credentials {
    store: Arc<Store>,
    service: String,
}

impl Credentials {
    pub(super) fn open(service: String) -> Result<Self, PlatformError> {
        let store = Store::new()?;

        Ok(Self { store, service })
    }

    fn entry(&self, name: &str) -> Result<Entry, PlatformError> {
        let modifiers = HashMap::from([(PERSISTENCE, THIS_MACHINE)]);

        let entry = self.store.build(&self.service, name, Some(&modifiers))?;

        Ok(entry)
    }
}

impl Vault for Credentials {
    fn place(&self) -> KeyPlace {
        KeyPlace::CredentialManager
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        match self.entry(name)?.get_secret() {
            Ok(secret) => Ok(Some(Zeroizing::new(secret))),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        Ok(self.entry(name)?.set_secret(bytes)?)
    }

    fn remove(&self, name: &str) -> Result<(), PlatformError> {
        match self.entry(name)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
