mod builder;
mod connection;
mod lease;
mod registry;
mod slot;

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::Arc;

pub use builder::PoolBuilder;
pub use connection::{ConnectionState, ConnectionStatus};
pub use lease::Lease;
use registry::Registry;
use slot::Slot;

use crate::exec::Execution;
use crate::known_hosts::KnownHosts;
use crate::{ExecLimits, ExecOutput, SshError, Target};

#[derive(Clone)]
pub struct Pool {
    registry: Arc<Registry>,
}

impl Pool {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn builder() -> PoolBuilder {
        PoolBuilder::new()
    }

    pub async fn acquire(&self, target: &Target) -> Result<Lease, SshError> {
        let lease = self.registry.lease(target);
        lease.connection().await?;

        Ok(lease)
    }

    pub async fn warm(&self, target: &Target) -> Result<ConnectionState, SshError> {
        let directory = self.registry.connector().directory();
        KnownHosts::load(directory).await?.require_known(target)?;

        let lease = self.acquire(target).await?;
        lease.keep_spare_session().await?;

        Ok(lease.state())
    }

    pub async fn exec(
        &self,
        target: &Target,
        command: &str,
        limits: ExecLimits,
        stop: impl Future<Output = ()>,
    ) -> Result<ExecOutput, SshError> {
        Execution::run(self, target, command, limits, stop).await
    }

    pub fn connections(&self) -> Vec<ConnectionStatus> {
        self.registry.statuses()
    }

    pub fn forget_vault_key(&self, name: &str) -> usize {
        self.registry.forget(|target| target.uses_vault_key(name))
    }

    fn from_registry(registry: Arc<Registry>) -> Self {
        Self { registry }
    }
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for Pool {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Pool")
            .field("connections", &self.connections().len())
            .field("grace", &self.registry.grace())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn forgetting_a_vault_key_drops_the_connections_that_use_it() {
        let pool = Pool::new();
        let keyed = Target::new("keyed.example", 22, "root").with_vault_key("night");
        let jumped = Target::new("inner.example", 22, "root").through(vec![keyed.clone()]);
        let plain = Target::new("plain.example", 22, "root");

        drop(pool.registry.lease(&keyed));
        drop(pool.registry.lease(&jumped));
        drop(pool.registry.lease(&plain));

        assert_eq!(pool.forget_vault_key("day"), 0);
        assert_eq!(pool.forget_vault_key("night"), 2);

        let left: Vec<String> = pool
            .connections()
            .into_iter()
            .map(|status| status.target)
            .collect();
        assert_eq!(left, vec!["root@plain.example".to_string()]);
    }
}
