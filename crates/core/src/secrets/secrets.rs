use std::sync::{Arc, Mutex, MutexGuard};

use synt_extensions::SecretSource;
use synt_platform::Vault;
use zeroize::Zeroizing;

use super::{SecretInfo, SecretName};
use crate::CoreError;
use crate::clock::unix_millis;
use crate::locked::locked;
use crate::state::State;

const LARGEST_SECRET: usize = 16 * 1024;

#[derive(Clone)]
pub struct Secrets {
    vault: Arc<dyn Vault>,
    state: State,
    changes: Arc<Mutex<()>>,
}

impl Secrets {
    pub fn new(vault: Arc<dyn Vault>, state: State) -> Self {
        Self {
            vault,
            state,
            changes: Arc::new(Mutex::new(())),
        }
    }

    pub fn put(&self, name: &SecretName, value: &[u8]) -> Result<SecretInfo, CoreError> {
        let _change = self.change();

        if value.is_empty() || value.len() > LARGEST_SECRET {
            return Err(CoreError::Secret(format!(
                "a secret holds 1 to {LARGEST_SECRET} bytes"
            )));
        }

        if self.state.ssh_key(name.as_str())?.is_some() {
            return Err(CoreError::Secret(format!(
                "secret://{name} already names an SSH key; pick another name"
            )));
        }

        let info = SecretInfo {
            name: name.to_string(),
            reference: name.reference(),
            bytes: u64::try_from(value.len()).unwrap_or(u64::MAX),
            created_at: u64::try_from(unix_millis()).unwrap_or_default(),
        };

        self.vault.write(&name.vault_item(), value)?;
        self.state.put_secret(&info)?;

        Ok(info)
    }

    pub fn list(&self) -> Result<Vec<SecretInfo>, CoreError> {
        self.state.secrets()
    }

    pub fn remove(&self, name: &SecretName) -> Result<SecretInfo, CoreError> {
        let _change = self.change();

        let Some(info) = self.state.secret(name.as_str())? else {
            return Err(CoreError::Secret(format!("there is no secret {name}")));
        };

        self.vault.remove(&name.vault_item())?;
        self.state.delete_secret(name.as_str())?;

        Ok(info)
    }

    fn change(&self) -> MutexGuard<'_, ()> {
        locked(&self.changes)
    }
}

impl SecretSource for Secrets {
    fn reveal(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
        let name = SecretName::parse(name)?;

        if self
            .state
            .secret(name.as_str())
            .map_err(|error| error.to_string())?
            .is_none()
        {
            return Ok(None);
        }

        self.vault
            .read(&name.vault_item())
            .map_err(|error| error.to_string())
    }
}
