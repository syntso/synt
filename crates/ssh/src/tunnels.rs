use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;

use super::{Destination, Tunnel, TunnelId, TunnelStatus};
use crate::locked::locked;
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
        stop: CancellationToken,
    ) -> Result<TunnelStatus, SshError> {
        let id = TunnelId::from(self.issued.fetch_add(1, Ordering::Relaxed) + 1);

        let tunnel = Tunnel::open(
            id,
            &self.pool,
            target,
            destination,
            local_port,
            stop.clone(),
        )
        .await?;
        let status = tunnel.status();

        locked(&self.open).insert(id, tunnel);
        self.close_when(id, stop);

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

    fn close_when(&self, id: TunnelId, stop: CancellationToken) {
        let tunnels = self.clone();

        tokio::spawn(async move {
            stop.cancelled().await;
            tunnels.close(id).await;
        });
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
