use std::fmt::{self, Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use super::{Pool, Slots};
use crate::KeyVault;
use crate::session::Connector;
use crate::ssh_directory::SshDirectory;

const GRACE: Duration = Duration::from_secs(30);

pub struct PoolBuilder {
    grace: Duration,
    vault: Option<Arc<dyn KeyVault>>,
    directory: SshDirectory,
}

impl PoolBuilder {
    pub(super) fn new() -> Self {
        Self {
            grace: GRACE,
            vault: None,
            directory: SshDirectory::User,
        }
    }

    #[must_use]
    pub fn grace(mut self, grace: Duration) -> Self {
        self.grace = grace;
        self
    }

    #[must_use]
    pub fn key_vault(mut self, vault: Arc<dyn KeyVault>) -> Self {
        self.vault = Some(vault);
        self
    }

    #[must_use]
    pub fn ssh_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = SshDirectory::At(directory.into());
        self
    }

    pub fn build(self) -> Pool {
        let connector = Connector::new(self.vault, self.directory);

        Pool::from_slots(Slots::new(self.grace, connector))
    }
}

impl Debug for PoolBuilder {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PoolBuilder")
            .field("grace", &self.grace)
            .field("key_vault", &self.vault.is_some())
            .field("ssh_directory", &self.directory)
            .finish()
    }
}
