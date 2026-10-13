use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::{Entry, Error};
use windows_native_keyring_store::Store;
use zeroize::Zeroizing;

use super::place::KeyPlace;
use super::vault::Vault;
use crate::PlatformError;

const PERSISTENCE: &str = "persistence";
const THIS_MACHINE: &str = "Local";

pub(crate) struct Credentials {
    store: Arc<Store>,
    service: String,
}

impl Credentials {
    pub(crate) fn open(service: String) -> Result<Self, PlatformError> {
        let store = Store::new().map_err(credentials)?;

        Ok(Self { store, service })
    }

    fn entry(&self, name: &str) -> Result<Entry, PlatformError> {
        let modifiers = HashMap::from([(PERSISTENCE, THIS_MACHINE)]);

        self.store
            .build(&self.service, name, Some(&modifiers))
            .map_err(credentials)
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
            Err(error) => Err(credentials(error)),
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        self.entry(name)?.set_secret(bytes).map_err(credentials)
    }
}

fn credentials(error: impl Display) -> PlatformError {
    PlatformError::Credentials(error.to_string())
}
