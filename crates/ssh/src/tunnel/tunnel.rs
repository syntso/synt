use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::forward::serve;
use super::{Destination, Traffic, TunnelId, TunnelStatus};
use crate::{Lease, Pool, SshError, Target};

const CONNECT_LIMIT: Duration = Duration::from_secs(30);

pub(crate) struct Tunnel {
    id: TunnelId,
    lease: Arc<Lease>,
    local_port: u16,
    destination: Destination,
    traffic: Arc<Traffic>,
    serving: JoinHandle<()>,
    stop: CancellationToken,
}

impl Tunnel {
    pub(crate) async fn open(
        id: TunnelId,
        pool: &Pool,
        target: &Target,
        destination: Destination,
        local_port: Option<u16>,
        stop: CancellationToken,
    ) -> Result<Self, SshError> {
        let listener = listen(local_port.unwrap_or(0)).await?;
        let local_port = listener.local_addr()?.port();

        let lease = time::timeout(CONNECT_LIMIT, pool.acquire(target))
            .await
            .map_err(|_| SshError::Timeout(CONNECT_LIMIT))??;
        let lease = Arc::new(lease);
        let traffic = Arc::new(Traffic::default());

        let serving = tokio::spawn(serve(
            listener,
            Arc::clone(&lease),
            destination.clone(),
            Arc::clone(&traffic),
        ));

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

    pub(crate) fn login_user(&self) -> &str {
        self.lease.target().user()
    }

    pub(crate) fn status(&self) -> TunnelStatus {
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

    pub(crate) async fn close(mut self) -> TunnelStatus {
        self.serving.abort();
        let _stopped = (&mut self.serving).await;

        self.status()
    }
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        self.serving.abort();
        self.stop.cancel();
    }
}

async fn listen(port: u16) -> Result<TcpListener, SshError> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|error| SshError::Listen {
            port,
            reason: error.to_string(),
        })
}
