use security_framework::base::Error;
use security_framework::passwords::{
    PasswordOptions, delete_generic_password_options, generic_password,
    set_generic_password_options,
};
use zeroize::Zeroizing;

use super::place::KeyPlace;
use super::vault::Vault;
use super::weak_vault::{WEAK_VAULT_FLAG, WeakVault};
use crate::PlatformError;

const PROBE: &str = "probe";
const ITEM_NOT_FOUND: i32 = -25300;
const MISSING_ENTITLEMENT: i32 = -34018;

pub(crate) struct Keychain {
    service: String,
    protected: bool,
}

impl Keychain {
    pub(crate) fn open(service: String, weak: WeakVault) -> Result<Self, PlatformError> {
        let protected = Self {
            service,
            protected: true,
        };

        match protected.probe() {
            Ok(()) => Ok(protected),
            Err(error) if error.code() == MISSING_ENTITLEMENT && weak.allowed() => Ok(Self {
                protected: false,
                ..protected
            }),
            Err(error) if error.code() == MISSING_ENTITLEMENT => Err(PlatformError::WeakVault {
                place: KeyPlace::LoginKeychain,
                flag: WEAK_VAULT_FLAG,
            }),
            Err(error) => Err(error.into()),
        }
    }

    fn probe(&self) -> Result<(), Error> {
        set_generic_password_options(&[], self.options(PROBE))?;
        delete_generic_password_options(self.options(PROBE))
    }

    fn find(&self, account: &str) -> Result<Option<Zeroizing<Vec<u8>>>, Error> {
        match generic_password(self.options(account)) {
            Ok(bytes) => Ok(Some(Zeroizing::new(bytes))),
            Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn options(&self, account: &str) -> PasswordOptions {
        let mut options = PasswordOptions::new_generic_password(&self.service, account);

        if self.protected {
            options.use_protected_keychain();
        }

        options
    }
}

impl Vault for Keychain {
    fn place(&self) -> KeyPlace {
        if self.protected {
            return KeyPlace::ProtectedKeychain;
        }

        KeyPlace::LoginKeychain
    }

    fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        Ok(self.find(name)?)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
        Ok(set_generic_password_options(bytes, self.options(name))?)
    }
}
