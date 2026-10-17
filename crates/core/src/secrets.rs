mod info;
mod name;
mod table;

pub use info::SecretInfo;
pub use name::{SecretItem, SecretName};
pub(crate) use table::SecretTable;

use std::error::Error;
use std::str;
use std::sync::Arc;

use synt_extensions::SecretSource;
use synt_journal::{Clock, SystemClock};
use synt_platform::Vault;
use zeroize::Zeroizing;

use crate::CoreError;
use crate::ssh_keys::SshKeyTable;
use crate::state::State;
use crate::vault::VaultItems;

const LARGEST_SECRET: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct Secrets {
    items: VaultItems<SecretItem>,
    table: SecretTable,
    keys: SshKeyTable,
}

impl Secrets {
    pub fn new(vault: Arc<dyn Vault>, state: State) -> Self {
        Self {
            items: VaultItems::new(vault),
            table: SecretTable::new(state.clone()),
            keys: SshKeyTable::new(state),
        }
    }

    pub fn put(&self, name: &SecretName, value: &[u8]) -> Result<SecretInfo, CoreError> {
        let _change = self.items.change();

        if value.is_empty() || value.len() > LARGEST_SECRET {
            return Err(CoreError::SecretSize(LARGEST_SECRET));
        }

        if self.keys.find(&name.recast())?.is_some() {
            return Err(CoreError::SecretNamesKey(name.clone()));
        }

        let info = SecretInfo::new(name, value.len(), SystemClock.now());

        self.items.write(name, value)?;
        self.table.put(&info)?;
        self.table.forget_use(name)?;

        Ok(info)
    }

    pub fn list(&self) -> Result<Vec<SecretInfo>, CoreError> {
        self.table.all()
    }

    pub fn remove(&self, name: &SecretName) -> Result<SecretInfo, CoreError> {
        let _change = self.items.change();

        let info = self
            .table
            .find(name)?
            .ok_or_else(|| CoreError::NoSecret(name.clone()))?;

        self.items.remove(name)?;
        self.table.delete(name)?;

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
        let bytes = self
            .items
            .read(name)?
            .ok_or_else(|| CoreError::SecretNotInVault(name.clone()))?;

        let text = str::from_utf8(&bytes).map_err(|_| CoreError::SecretNotText(name.clone()))?;

        Ok(Zeroizing::new(text.to_owned()))
    }

    fn reveal_to(
        &self,
        name: &SecretName,
        extension: &str,
    ) -> Result<Option<Zeroizing<Vec<u8>>>, CoreError> {
        if self.table.find(name)?.is_none() {
            return Ok(None);
        }

        let value = self.items.read(name)?;

        if value.is_some() {
            self.claim(name, &format!("the extension {extension}"))?;
        }

        Ok(value)
    }

    fn claim(&self, name: &SecretName, used_for: &str) -> Result<(), CoreError> {
        let _change = self.items.change();

        match self.table.use_of(name)? {
            None => self.table.bind_use(name, used_for),
            Some(bound) if bound == used_for => Ok(()),
            Some(bound) => Err(CoreError::SecretKeptFor {
                name: name.clone(),
                bound,
                used_for: used_for.to_owned(),
            }),
        }
    }
}

impl SecretSource for Secrets {
    fn reveal(
        &self,
        name: &str,
        extension: &str,
    ) -> Result<Option<Zeroizing<Vec<u8>>>, Box<dyn Error + Send + Sync>> {
        let name = SecretName::parse(name)?;

        Ok(self.reveal_to(&name, extension)?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use synt_extensions::SecretSource;

    use super::{SecretName, Secrets};
    use crate::state::State;
    use crate::vault::MemoryVault;

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
        let refused = secrets
            .reveal("grafana/token", "grafana")
            .unwrap_err()
            .to_string();

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
