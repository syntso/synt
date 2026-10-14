use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

use super::{ConnectionState, ConnectionStatus, Lease, Slots};
use crate::known_hosts::KnownHosts;
use crate::session::Connector;
use crate::{KeyVault, SshError, Target};

const GRACE: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Pool {
    slots: Arc<Slots>,
}

impl Pool {
    pub fn new() -> Self {
        Self::with_grace(GRACE)
    }

    pub fn with_grace(grace: Duration) -> Self {
        Self {
            slots: Slots::new(grace, Connector::default()),
        }
    }

    pub fn with_key_vault(vault: Arc<dyn KeyVault>) -> Self {
        Self {
            slots: Slots::new(GRACE, Connector::with_vault(vault)),
        }
    }

    pub async fn acquire(&self, target: &Target) -> Result<Lease, SshError> {
        let lease = self.slots.lease(target);
        lease.connection().await?;

        Ok(lease)
    }

    pub async fn warm(&self, target: &Target) -> Result<ConnectionState, SshError> {
        refuse_unknown_hops(target).await?;

        let lease = self.acquire(target).await?;
        lease.keep_spare_session().await?;

        Ok(lease.state())
    }

    pub fn connections(&self) -> Vec<ConnectionStatus> {
        self.slots.statuses()
    }

    pub fn forget_vault_key(&self, name: &str) -> usize {
        self.slots.forget(|target| target.uses_vault_key(name))
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
            .field("grace", &self.slots.grace())
            .finish()
    }
}

async fn refuse_unknown_hops(target: &Target) -> Result<(), SshError> {
    let known_hosts = KnownHosts::load().await?;

    let unknown = hops(target).into_iter().find(|hop| {
        known_hosts
            .known_algorithms(hop.host(), hop.port())
            .is_empty()
    });

    match unknown {
        Some(hop) => Err(SshError::NotKnownHost {
            host: hop.address(),
        }),
        None => Ok(()),
    }
}

fn hops(target: &Target) -> Vec<&Target> {
    let mut all: Vec<&Target> = target.jumps().iter().flat_map(hops).collect();
    all.push(target);

    all
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

        drop(pool.slots.lease(&keyed));
        drop(pool.slots.lease(&jumped));
        drop(pool.slots.lease(&plain));

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
