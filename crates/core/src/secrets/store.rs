use std::error::Error;
use std::fmt::{self, Debug, Formatter};
use std::str;
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
        self.state.forget_secret_use(name.as_str())?;

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

    pub(crate) fn read_text_for(
        &self,
        name: &SecretName,
        used_for: &str,
    ) -> Result<Zeroizing<String>, CoreError> {
        let text = self.read_text(name)?;
        self.claim(name, used_for)?;

        Ok(text)
    }

    fn read_text(&self, name: &SecretName) -> Result<Zeroizing<String>, CoreError> {
        let missing = || {
            CoreError::Secret(format!(
                "secret {}: there is no such secret in the vault; store it with secret.put",
                name.reference()
            ))
        };

        let bytes = self.vault.read(&name.vault_item())?.ok_or_else(missing)?;
        let text = str::from_utf8(&bytes).map_err(|_| {
            CoreError::Secret(format!("secret {} is not UTF-8 text", name.reference()))
        })?;

        Ok(Zeroizing::new(text.to_owned()))
    }

    fn claim(&self, name: &SecretName, used_for: &str) -> Result<(), CoreError> {
        let _change = self.change();

        match self.state.secret_use(name.as_str())? {
            None => self.state.bind_secret_use(name.as_str(), used_for),
            Some(bound) if bound == used_for => Ok(()),
            Some(bound) => Err(CoreError::Secret(format!(
                "secret {} is kept for {bound}, so it is not sent to {used_for}; if it \
                 should go there now, put it again with secret.put",
                name.reference()
            ))),
        }
    }

    fn change(&self) -> MutexGuard<'_, ()> {
        locked(&self.changes)
    }
}

impl Debug for Secrets {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("Secrets").finish_non_exhaustive()
    }
}

impl SecretSource for Secrets {
    fn reveal(
        &self,
        name: &str,
        extension: &str,
    ) -> Result<Option<Zeroizing<Vec<u8>>>, Box<dyn Error + Send + Sync>> {
        let name = SecretName::parse(name)?;

        if self
            .state
            .secret(name.as_str())
            .map_err(|error| error.to_string())?
            .is_none()
        {
            return Ok(None);
        }

        let value = self
            .vault
            .read(&name.vault_item())
            .map_err(|error| error.to_string())?;

        if value.is_some() {
            self.claim(&name, &format!("the extension {extension}"))
                .map_err(|error| error.to_string())?;
        }

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use synt_extensions::SecretSource;
    use synt_platform::{KeyPlace, PlatformError, Vault};
    use zeroize::Zeroizing;

    use super::Secrets;
    use crate::secrets::SecretName;
    use crate::state::State;

    #[derive(Default)]
    struct MemoryVault {
        items: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl Vault for MemoryVault {
        fn place(&self) -> KeyPlace {
            KeyPlace::Files("memory".into())
        }

        fn read(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
            Ok(self
                .items
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(Zeroizing::new))
        }

        fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
            self.items
                .lock()
                .unwrap()
                .insert(name.to_owned(), bytes.to_vec());
            Ok(())
        }

        fn remove(&self, name: &str) -> Result<(), PlatformError> {
            self.items.lock().unwrap().remove(name);
            Ok(())
        }
    }

    fn secrets() -> (Secrets, SecretName, tempfile::TempDir) {
        let folder = tempfile::tempdir().unwrap();
        let state = State::open(&folder.path().join("state.sqlite")).unwrap();
        let secrets = Secrets::new(Arc::new(MemoryVault::default()), state);
        let name = SecretName::parse("grafana/token").unwrap();

        secrets.put(&name, b"t0ken").unwrap();

        (secrets, name, folder)
    }

    #[test]
    fn a_secret_an_extension_used_is_not_sent_to_a_database() {
        let (secrets, name, _folder) = secrets();

        assert!(
            secrets
                .reveal("grafana/token", "grafana")
                .unwrap()
                .is_some()
        );
        let refused = secrets
            .read_text_for(&name, "the database postgres attacker:5432")
            .unwrap_err();

        assert!(
            refused
                .to_string()
                .contains("kept for the extension grafana"),
            "{refused}"
        );
    }

    #[test]
    fn a_database_password_is_not_handed_to_an_extension() {
        let (secrets, name, _folder) = secrets();

        secrets
            .read_text_for(&name, "the database postgres db:5432")
            .unwrap();
        let refused = secrets.reveal("grafana/token", "grafana").unwrap_err();

        assert!(
            refused.contains("kept for the database postgres db:5432"),
            "{refused}"
        );
    }

    #[test]
    fn the_first_use_stays_allowed_and_putting_again_frees_the_secret() {
        let (secrets, name, _folder) = secrets();

        secrets.reveal("grafana/token", "grafana").unwrap();
        secrets.reveal("grafana/token", "grafana").unwrap();
        assert!(secrets.reveal("grafana/token", "other").is_err());

        secrets.put(&name, b"t0ken").unwrap();

        assert!(secrets.reveal("grafana/token", "other").unwrap().is_some());
    }
}
