use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::{Destination, Tunnel, TunnelId, TunnelStatus};
use crate::pool::locked;
use crate::{Pool, SshError, Target};

#[derive(Clone)]
pub struct Tunnels {
    pool: Pool,
    open: Arc<Mutex<BTreeMap<TunnelId, Tunnel>>>,
    issued: Arc<AtomicU64>,
}

impl Tunnels {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            open: Arc::new(Mutex::new(BTreeMap::new())),
            issued: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn open(
        &self,
        target: &Target,
        destination: Destination,
        local_port: Option<u16>,
    ) -> Result<TunnelStatus, SshError> {
        let id = TunnelId::from(self.issued.fetch_add(1, Ordering::Relaxed) + 1);

        let tunnel = Tunnel::open(id, &self.pool, target, destination, local_port).await?;
        let status = tunnel.status();

        locked(&self.open).insert(id, tunnel);

        Ok(status)
    }

    pub fn list(&self) -> Vec<TunnelStatus> {
        locked(&self.open).values().map(Tunnel::status).collect()
    }

    pub fn login_user(&self, id: TunnelId) -> Option<String> {
        locked(&self.open)
            .get(&id)
            .map(|tunnel| tunnel.login_user().to_owned())
    }

    pub async fn close(&self, id: TunnelId) -> Option<TunnelStatus> {
        let tunnel = locked(&self.open).remove(&id)?;

        Some(tunnel.close().await)
    }
}

impl Debug for Tunnels {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Tunnels")
            .field("open", &locked(&self.open).len())
            .finish_non_exhaustive()
    }
}
