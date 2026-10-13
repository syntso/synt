use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

use super::{ConnectionState, ConnectionStatus, Lease, Slots};
use crate::known_hosts::KnownHosts;
use crate::{SshError, Target};

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
            slots: Slots::new(grace),
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
