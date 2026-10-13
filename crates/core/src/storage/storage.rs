use std::path::{Path, PathBuf};
use std::sync::Arc;

use synt_journal::{Journal, JournalBuilder, JournalError, JournalId, KeyStore};
use synt_platform::Vault;

use crate::CoreError;
use crate::grants::GrantSeal;
use crate::keys::VaultKeyStore;
use crate::state::State;

const JOURNALS: &str = "journals";
const JOURNAL_FILE: &str = "main.sqlite";
const JOURNAL_ITEM: &str = "journal.main";
const STATE: &str = "state";
const STATE_FILE: &str = "main.sqlite";

pub struct Storage {
    journals: PathBuf,
    state: PathBuf,
    vault: Arc<dyn Vault>,
}

impl Storage {
    pub fn prepare(data: &Path, vault: Box<dyn Vault>) -> Result<Self, CoreError> {
        let journals = data.join(JOURNALS);
        synt_platform::create_private_directory(&journals)?;

        let state = data.join(STATE);
        synt_platform::create_private_directory(&state)?;

        Ok(Self {
            journals,
            state,
            vault: Arc::from(vault),
        })
    }

    pub fn journal(&self) -> Result<Journal, CoreError> {
        let path = self.journals.join(JOURNAL_FILE);
        let keys = Arc::new(VaultKeyStore::new(Arc::clone(&self.vault)));
        let exists = path.try_exists()?;

        match self.remembered_id()? {
            Some(id) if exists => Ok(Journal::open(&path, id, keys)?),
            Some(id) => self.resume(&path, id, keys),
            None if exists => Err(CoreError::UnknownJournal {
                journal: path,
                place: self.vault.place(),
            }),
            None => self.create(&path, keys),
        }
    }

    pub fn state(&self) -> Result<State, CoreError> {
        State::open(&self.state.join(STATE_FILE))
    }

    pub fn grant_seal(&self) -> Result<GrantSeal, CoreError> {
        GrantSeal::open(Arc::clone(&self.vault))
    }

    fn create(&self, path: &Path, keys: Arc<VaultKeyStore>) -> Result<Journal, CoreError> {
        let id = JournalId::generate().map_err(|error| CoreError::Random(error.to_string()))?;
        self.vault.write(JOURNAL_ITEM, id.as_bytes())?;

        Ok(JournalBuilder::new(keys).id(id).create(path)?)
    }

    fn resume(
        &self,
        path: &Path,
        id: JournalId,
        keys: Arc<VaultKeyStore>,
    ) -> Result<Journal, CoreError> {
        if keys.anchors(id).map_err(JournalError::from)?.is_some() {
            return Err(CoreError::JournalMissing {
                id,
                journal: path.to_owned(),
                place: self.vault.place(),
            });
        }

        Ok(JournalBuilder::new(keys).id(id).create(path)?)
    }

    fn remembered_id(&self) -> Result<Option<JournalId>, CoreError> {
        let Some(bytes) = self.vault.read(JOURNAL_ITEM)? else {
            return Ok(None);
        };

        let bytes: [u8; 16] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| CoreError::DamagedKey(JOURNAL_ITEM.to_owned()))?;

        Ok(Some(JournalId::from_bytes(bytes)))
    }
}
