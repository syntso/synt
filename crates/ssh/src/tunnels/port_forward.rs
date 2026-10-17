use std::net::Ipv4Addr;
use std::num::NonZeroU16;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::{Destination, Forwarder, Traffic, TunnelId, TunnelStatus};
use crate::{Lease, Pool, SshError, Target};

const CONNECT_LIMIT: Duration = Duration::from_secs(30);

pub(super) struct PortForward {
    id: TunnelId,
    lease: Arc<Lease>,
    local_port: u16,
    destination: Destination,
    traffic: Arc<Traffic>,
    serving: JoinHandle<()>,
    stop: CancellationToken,
}

impl PortForward {
    pub(super) async fn open(
        id: TunnelId,
        pool: &Pool,
        target: &Target,
        destination: Destination,
        local_port: Option<NonZeroU16>,
        stop: CancellationToken,
    ) -> Result<Self, SshError> {
        let listener = Self::listen(local_port.map_or(0, NonZeroU16::get)).await?;
        let local_port = listener.local_addr()?.port();

        let lease = time::timeout(CONNECT_LIMIT, pool.acquire(target))
            .await
            .map_err(|_| SshError::Timeout(CONNECT_LIMIT))??;
        let lease = Arc::new(lease);
        let traffic = Arc::new(Traffic::default());

        let forwarder = Forwarder::new(
            Arc::clone(&lease),
            destination.clone(),
            Arc::clone(&traffic),
        );
        let serving = tokio::spawn(forwarder.serve(listener));

        Ok(Self {
            id,
            lease,
            local_port,
            destination,
            traffic,
            serving,
            stop,
        })
    }

    pub(super) fn login_user(&self) -> &str {
        self.lease.target().user()
    }

    pub(super) fn status(&self) -> TunnelStatus {
        TunnelStatus {
            id: self.id,
            target: self.lease.target().label(),
            local_port: self.local_port,
            destination: self.destination.clone(),
            state: self.lease.state(),
            active: self.traffic.active(),
            connections: self.traffic.connections(),
            sent_bytes: self.traffic.sent(),
            received_bytes: self.traffic.received(),
            last_error: self.traffic.last_error(),
        }
    }

    pub(super) async fn close(mut self) -> TunnelStatus {
        self.serving.abort();
        let _stopped = (&mut self.serving).await;

        self.status()
    }

    async fn listen(port: u16) -> Result<TcpListener, SshError> {
        TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|cause| SshError::Listen {
                port,
                cause: Arc::new(cause),
            })
    }
}

impl Drop for PortForward {
    fn drop(&mut self) {
        self.serving.abort();
        self.stop.cancel();
    }
}
