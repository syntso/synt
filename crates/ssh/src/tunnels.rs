mod destination;
mod forwarder;
mod id;
mod port_forward;
mod status;
mod traffic;

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroU16;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use synt_platform::Lock;
use tokio_util::sync::CancellationToken;

pub use destination::Destination;
use forwarder::Forwarder;
pub use id::TunnelId;
use port_forward::PortForward;
pub use status::TunnelStatus;
use traffic::Traffic;

use crate::{Pool, SshError, Target};

#[derive(Clone)]
pub struct Tunnels {
    pool: Pool,
    open: Arc<Mutex<BTreeMap<TunnelId, PortForward>>>,
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
        local_port: Option<NonZeroU16>,
        stop: CancellationToken,
    ) -> Result<TunnelStatus, SshError> {
        let id = TunnelId::from(self.issued.fetch_add(1, Ordering::Relaxed) + 1);

        let forward = PortForward::open(
            id,
            &self.pool,
            target,
            destination,
            local_port,
            stop.clone(),
        )
        .await?;
        let status = forward.status();

        self.open.locked().insert(id, forward);
        self.close_when(id, stop);

        Ok(status)
    }

    pub fn list(&self) -> Vec<TunnelStatus> {
        self.open
            .locked()
            .values()
            .map(PortForward::status)
            .collect()
    }

    pub fn logs_in_as_root(&self, id: TunnelId) -> bool {
        self.open
            .locked()
            .get(&id)
            .is_some_and(PortForward::logs_in_as_root)
    }

    pub async fn close(&self, id: TunnelId) -> Option<TunnelStatus> {
        let forward = self.open.locked().remove(&id)?;

        Some(forward.close().await)
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
            .field("open", &self.open.locked().len())
            .finish_non_exhaustive()
    }
}
