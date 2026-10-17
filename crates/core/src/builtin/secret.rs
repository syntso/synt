mod list;
mod put;
mod remove;

pub use list::SecretList;
pub use put::SecretPut;
pub use remove::SecretRemove;

use synt_actions::{ActionError, Registry};

use crate::secrets::Secrets;

#[derive(Debug, Clone)]
pub struct SecretActions {
    secrets: Secrets,
}

impl SecretActions {
    pub fn new(secrets: Secrets) -> Self {
        Self { secrets }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(SecretPut::new(self.secrets.clone()))?;
        registry.register(SecretList::new(self.secrets.clone()))?;
        registry.register(SecretRemove::new(self.secrets.clone()))
    }
}
