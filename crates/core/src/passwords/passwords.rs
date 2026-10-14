use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};

use synt_platform::Vault;
use zeroize::Zeroizing;

use super::{PasswordInfo, PasswordName};
use crate::CoreError;
use crate::clock::unix_millis;
use crate::locked::locked;
use crate::state::State;

#[derive(Clone)]
pub struct Passwords {
    vault: Arc<dyn Vault>,
    state: State,
    changes: Arc<Mutex<()>>,
}

impl Passwords {
    pub fn new(vault: Arc<dyn Vault>, state: State) -> Self {
        Self {
            vault,
            state,
            changes: Arc::new(Mutex::new(())),
        }
    }

    pub fn store(&self, name: &PasswordName, password: &str) -> Result<PasswordInfo, CoreError> {
        let _change = self.change();

        let info = PasswordInfo {
            name: name.to_string(),
            reference: name.reference(),
            updated_at: u64::try_from(unix_millis()).unwrap_or_default(),
        };

        self.vault.write(&name.vault_item(), password.as_bytes())?;
        self.state.save_password(&info)?;

        Ok(info)
    }

    pub fn list(&self) -> Result<Vec<PasswordInfo>, CoreError> {
        self.state.passwords()
    }

    pub fn remove(&self, name: &PasswordName) -> Result<PasswordInfo, CoreError> {
        let _change = self.change();

        let Some(info) = self.state.password(name.as_str())? else {
            return Err(CoreError::Password(format!(
                "there is no password named {name}"
            )));
        };

        self.vault.remove(&name.vault_item())?;
        self.state.delete_password(name.as_str())?;

        Ok(info)
    }

    pub(crate) fn read(&self, name: &PasswordName) -> Result<Zeroizing<String>, CoreError> {
        let missing = || {
            CoreError::Password(format!(
                "password {}: there is no such password in the vault; store it with password.set",
                name.reference()
            ))
        };

        let bytes = self.vault.read(&name.vault_item())?.ok_or_else(missing)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            CoreError::Password(format!("password {} is not UTF-8 text", name.reference()))
        })?;

        Ok(Zeroizing::new(text.to_owned()))
    }

    fn change(&self) -> MutexGuard<'_, ()> {
        locked(&self.changes)
    }
}

impl Debug for Passwords {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Passwords").finish_non_exhaustive()
    }
}
