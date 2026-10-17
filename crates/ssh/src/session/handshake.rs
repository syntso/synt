use std::sync::{Arc, OnceLock};
use std::time::Duration;

use russh::client::{self, Config, Handle};
use russh::keys::Algorithm;
use tokio::io::{AsyncRead, AsyncWrite};

use super::{Algorithms, ClientHandler, Policy};
use crate::known_hosts::KnownHosts;
use crate::{SshError, Target};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const KEEPALIVE_MISSES: usize = 3;
const WINDOW_BYTES: u32 = 2 * 1024 * 1024;
const PACKET_BYTES: u32 = 32 * 1024;

pub(super) struct Handshake {
    hop: Target,
    config: Arc<Config>,
    known_hosts: Arc<KnownHosts>,
    negotiated: Arc<OnceLock<Algorithms>>,
}

impl Handshake {
    pub(super) fn new(hop: &Target, known_hosts: KnownHosts) -> Self {
        let known = known_hosts.known_algorithms(hop.host(), hop.port());

        Self {
            hop: hop.clone(),
            config: Arc::new(Self::config(&known)),
            known_hosts: Arc::new(known_hosts),
            negotiated: Arc::new(OnceLock::new()),
        }
    }

    pub(super) async fn over<S>(&self, stream: S) -> Result<Handle<ClientHandler>, SshError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let handler = ClientHandler::new(
            &self.hop,
            Arc::clone(&self.known_hosts),
            Arc::clone(&self.negotiated),
        );

        client::connect_stream(Arc::clone(&self.config), stream, handler)
            .await
            .map_err(|error| match error {
                SshError::Protocol(protocol) => SshError::connect(self.hop.address(), protocol),
                rejection => rejection,
            })
    }

    pub(super) fn negotiated(&self) -> Arc<OnceLock<Algorithms>> {
        Arc::clone(&self.negotiated)
    }

    fn config(known: &[Algorithm]) -> Config {
        Config {
            nodelay: true,
            keepalive_interval: Some(KEEPALIVE_INTERVAL),
            keepalive_max: KEEPALIVE_MISSES,
            window_size: WINDOW_BYTES,
            maximum_packet_size: PACKET_BYTES,
            preferred: Policy::new(known).preferred(),
            ..Config::default()
        }
    }
}
