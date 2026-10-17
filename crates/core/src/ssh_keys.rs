mod info;
mod name;
mod table;

pub use info::KeyInfo;
pub use name::{KeyName, SshKeyItem};
pub(crate) use table::SshKeyTable;

use std::sync::Arc;

use synt_journal::{Clock, SystemClock};
use synt_platform::Vault;
use synt_ssh::{KeyVault, StoredKey, VaultError};
use zeroize::Zeroizing;

use crate::CoreError;
use crate::secrets::SecretTable;
use crate::state::State;
use crate::vault::VaultItems;

#[derive(Debug, Clone)]
pub struct SshKeys {
    items: VaultItems<SshKeyItem>,
    keys: SshKeyTable,
    secrets: SecretTable,
}

impl SshKeys {
    pub fn new(vault: Arc<dyn Vault>, state: State) -> Self {
        Self {
            items: VaultItems::new(vault),
            keys: SshKeyTable::new(state.clone()),
            secrets: SecretTable::new(state),
        }
    }

    pub fn store(&self, name: &KeyName, key: &StoredKey) -> Result<KeyInfo, CoreError> {
        let _change = self.items.change();

        self.check_free(name)?;

        let info = KeyInfo::new(name, key.summary(), SystemClock.now());
        self.items.write(name, key.private_bytes())?;

        if let Err(error) = self.keys.insert(&info) {
            self.items.remove(name)?;
            return Err(error);
        }

        Ok(info)
    }

    pub fn list(&self) -> Result<Vec<KeyInfo>, CoreError> {
        self.keys.all()
    }

    pub fn remove(&self, name: &KeyName) -> Result<KeyInfo, CoreError> {
        let _change = self.items.change();

        let info = self
            .keys
            .find(name)?
            .ok_or_else(|| CoreError::NoKey(name.clone()))?;

        self.items.remove(name)?;
        self.keys.delete(name)?;

        Ok(info)
    }

    fn check_free(&self, name: &KeyName) -> Result<(), CoreError> {
        if self.keys.find(name)?.is_some() {
            return Err(CoreError::KeyExists(name.clone()));
        }

        if self.secrets.find(&name.recast())?.is_some() {
            return Err(CoreError::KeyNamesSecret(name.clone()));
        }

        Ok(())
    }
}

impl KeyVault for SshKeys {
    fn private_key(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, VaultError> {
        let name = KeyName::parse(name)?;

        Ok(self.items.read(&name)?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use synt_ssh::{KeyVault, StoredKey};
    use synt_testkit::MemoryVault;

    use super::{KeyName, SshKeys};
    use crate::state::State;

    fn keys() -> (SshKeys, tempfile::TempDir) {
        let folder = tempfile::tempdir().expect("folder");
        let state = State::open(&folder.path().join("state.sqlite")).expect("state");

        (
            SshKeys::new(Arc::new(MemoryVault::default()), state),
            folder,
        )
    }

    #[test]
    fn a_stored_key_is_listed_by_its_public_half_and_read_back_whole() {
        let (keys, _folder) = keys();
        let name = KeyName::parse("prod/deploy").expect("name");
        let key = StoredKey::generate_ed25519("test").expect("generate");

        let info = keys.store(&name, &key).expect("store");
        let listed = keys.list().expect("list");
        let private = keys
            .private_key("prod/deploy")
            .expect("read")
            .expect("present");

        assert_eq!(listed, vec![info.clone()]);
        assert_eq!(info.reference, "secret://prod/deploy");
        assert!(
            !serde_json::to_string(&listed)
                .expect("json")
                .contains("PRIVATE KEY")
        );
        assert_eq!(private.as_slice(), key.private_bytes());
    }

    #[test]
    fn two_stores_of_one_name_at_once_keep_one_whole_key() {
        let (keys, _folder) = keys();
        let name = KeyName::parse("race").expect("name");
        let start = Arc::new(Barrier::new(8));

        let racers: Vec<_> = (0..8)
            .map(|_| {
                let keys = keys.clone();
                let name = name.clone();
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    let key = StoredKey::generate_ed25519("race").expect("generate");
                    start.wait();
                    keys.store(&name, &key).ok().map(|info| (info, key))
                })
            })
            .collect();
        let winners: Vec<_> = racers
            .into_iter()
            .filter_map(|racer| racer.join().expect("racer"))
            .collect();

        let [(info, key)] = winners.as_slice() else {
            panic!("exactly one store wins");
        };
        let private = keys.private_key("race").expect("read").expect("present");

        assert_eq!(keys.list().expect("list"), vec![info.clone()]);
        assert_eq!(private.as_slice(), key.private_bytes());
    }

    #[test]
    fn a_name_is_taken_once_and_freed_by_removing_it() {
        let (keys, _folder) = keys();
        let name = KeyName::parse("night").expect("name");
        let key = StoredKey::generate_ed25519("test").expect("generate");

        keys.store(&name, &key).expect("store");
        assert!(keys.store(&name, &key).is_err());

        keys.remove(&name).expect("remove");

        assert!(keys.list().expect("list").is_empty());
        assert!(keys.private_key("night").expect("read").is_none());
        assert!(keys.remove(&name).is_err());
    }
}
