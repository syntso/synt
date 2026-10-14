use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use synt_platform::Vault;
use synt_ssh::{KeyVault, StoredKey};
use zeroize::Zeroizing;

use super::{KeyInfo, KeyName};
use crate::CoreError;
use crate::locked::locked;
use crate::state::State;

#[derive(Clone)]
pub struct SshKeys {
    vault: Arc<dyn Vault>,
    state: State,
    changes: Arc<Mutex<()>>,
}

impl SshKeys {
    pub fn new(vault: Arc<dyn Vault>, state: State) -> Self {
        Self {
            vault,
            state,
            changes: Arc::new(Mutex::new(())),
        }
    }

    pub fn store(&self, name: &KeyName, key: &StoredKey) -> Result<KeyInfo, CoreError> {
        let _change = self.change();

        if self.state.ssh_key(name.as_str())?.is_some() {
            return Err(CoreError::SshKey(format!(
                "there is already a key named {name}; remove it first"
            )));
        }

        let info = KeyInfo::new(name, key.summary(), now());
        self.vault.write(&name.vault_item(), key.private_bytes())?;

        if let Err(error) = self.state.insert_ssh_key(&info) {
            self.vault.remove(&name.vault_item())?;
            return Err(error);
        }

        Ok(info)
    }

    pub fn list(&self) -> Result<Vec<KeyInfo>, CoreError> {
        self.state.ssh_keys()
    }

    pub fn remove(&self, name: &KeyName) -> Result<KeyInfo, CoreError> {
        let _change = self.change();

        let Some(info) = self.state.ssh_key(name.as_str())? else {
            return Err(CoreError::SshKey(format!("there is no key named {name}")));
        };

        self.vault.remove(&name.vault_item())?;
        self.state.delete_ssh_key(name.as_str())?;

        Ok(info)
    }

    fn change(&self) -> MutexGuard<'_, ()> {
        locked(&self.changes)
    }
}

impl KeyVault for SshKeys {
    fn private_key(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, String> {
        let name = KeyName::parse(name)?;

        self.vault
            .read(&name.vault_item())
            .map_err(|error| error.to_string())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use synt_platform::{KeyPlace, PlatformError, Vault};
    use synt_ssh::{KeyVault, StoredKey};
    use zeroize::Zeroizing;

    use super::SshKeys;
    use crate::ssh_keys::KeyName;
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
            let items = self.items.lock().expect("lock");
            Ok(items.get(name).cloned().map(Zeroizing::new))
        }

        fn write(&self, name: &str, bytes: &[u8]) -> Result<(), PlatformError> {
            let mut items = self.items.lock().expect("lock");
            items.insert(name.to_owned(), bytes.to_vec());
            Ok(())
        }

        fn remove(&self, name: &str) -> Result<(), PlatformError> {
            self.items.lock().expect("lock").remove(name);
            Ok(())
        }
    }

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
        let start = Arc::new(std::sync::Barrier::new(8));

        let racers: Vec<_> = (0..8)
            .map(|_| {
                let keys = keys.clone();
                let name = name.clone();
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
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
