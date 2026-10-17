mod generate;
mod import;
mod kind;
mod list;
mod remove;

pub use generate::KeyGenerate;
pub use import::KeyImport;
pub(crate) use kind::KeyKind;
pub use list::KeyList;
pub use remove::KeyRemove;

use synt_actions::{ActionError, ActionGroup, Registry};
use synt_ssh::Pool;

use crate::ssh_keys::SshKeys;

#[derive(Debug, Clone)]
pub struct KeyActions {
    keys: SshKeys,
    pool: Pool,
}

impl KeyActions {
    pub fn new(keys: SshKeys, pool: Pool) -> Self {
        Self { keys, pool }
    }
}

impl ActionGroup for KeyActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(KeyImport::new(self.keys.clone()))?;
        registry.register(KeyGenerate::new(self.keys.clone()))?;
        registry.register(KeyList::new(self.keys.clone()))?;
        registry.register(KeyRemove::new(self.keys.clone(), self.pool.clone()))
    }
}
