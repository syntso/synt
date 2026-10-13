use std::fmt::{self, Debug, Formatter};
use std::sync::Arc;
use std::time::Duration;

use super::{ConnectionStatus, Lease, Slots};
use crate::{SshError, Target};

const GRACE: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct Pool {
    slots: Arc<Slots>,
}

impl Pool {
    pub fn new() -> Self {
        Self {
            slots: Slots::new(GRACE),
        }
    }

    pub async fn acquire(&self, target: &Target) -> Result<Lease, SshError> {
        let lease = self.slots.lease(target);
        lease.connection().await?;

        Ok(lease)
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
            .finish()
    }
}
