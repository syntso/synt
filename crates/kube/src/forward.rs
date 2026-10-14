mod backend;
mod relay;
mod request;
mod status;
mod target;
mod traffic;

use backend::Backend;
use relay::Relay;
pub use request::ForwardRequest;
pub use status::ForwardStatus;
pub use target::ForwardTarget;
use traffic::ForwardTraffic;

use std::fmt::{self, Debug, Formatter};
use std::net::Ipv4Addr;
use std::num::NonZeroU16;
use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::KubeError;

const ANY_PORT: u16 = 0;

pub struct PortForward {
    target: String,
    namespace: String,
    pod: String,
    remote_port: u16,
    local_port: u16,
    traffic: Arc<ForwardTraffic>,
    serving: JoinHandle<()>,
}

impl PortForward {
    async fn start(
        target: String,
        namespace: String,
        relay: Relay,
        local_port: Option<NonZeroU16>,
    ) -> Result<Self, KubeError> {
        let listener = Self::listen(local_port.map_or(ANY_PORT, NonZeroU16::get)).await?;
        let local_port = listener.local_addr()?.port();

        let pod = relay.pod().to_owned();
        let remote_port = relay.port();
        let traffic = relay.traffic();
        let serving = tokio::spawn(relay.serve(listener));

        Ok(Self {
            target,
            namespace,
            pod,
            remote_port,
            local_port,
            traffic,
            serving,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn status(&self) -> ForwardStatus {
        ForwardStatus {
            target: self.target.clone(),
            namespace: self.namespace.clone(),
            pod: self.pod.clone(),
            remote_port: self.remote_port,
            local_port: self.local_port,
            connections: self.traffic.connections(),
            active: self.traffic.active(),
            sent_bytes: self.traffic.sent(),
            received_bytes: self.traffic.received(),
            last_error: self.traffic.last_error(),
        }
    }

    pub async fn close(mut self) -> ForwardStatus {
        self.serving.abort();
        let _stopped = (&mut self.serving).await;

        self.status()
    }

    async fn listen(port: u16) -> Result<TcpListener, KubeError> {
        TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|error| KubeError::Io(format!("cannot listen on 127.0.0.1:{port}: {error}")))
    }
}

impl Drop for PortForward {
    fn drop(&mut self) {
        self.serving.abort();
    }
}

impl Debug for PortForward {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortForward")
            .field("target", &self.target)
            .field("local_port", &self.local_port)
            .finish_non_exhaustive()
    }
}
