use std::fmt::Display;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use russh::client::{self, Config, Handle, Msg};
use russh::keys::Algorithm;
use russh::{Channel, Disconnect};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time;

use super::{ClientHandler, HandshakeError};
use crate::auth::Authenticator;
use crate::known_hosts::{KnownHosts, same_family};
use crate::{SshError, Target};

const CLOSE_LIMIT: Duration = Duration::from_secs(5);
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);
const KEEPALIVE_MISSES: usize = 3;
const ORIGINATOR_ADDRESS: &str = "127.0.0.1";
const ORIGINATOR_PORT: u32 = 0;
const LANGUAGE_TAG: &str = "en";

#[derive(Clone)]
pub(crate) struct Connection {
    handle: Arc<Handle<ClientHandler>>,
}

impl Connection {
    pub(crate) async fn open(hop: &Target, through: Option<&Connection>) -> Result<Self, SshError> {
        let known_hosts = Arc::new(KnownHosts::load().await?);
        let config = Arc::new(client_config(&known_hosts, hop));

        let handle = match through {
            None => dial(hop, &config, &known_hosts).await?,
            Some(previous) => tunnel(&previous.handle, hop, &config, &known_hosts).await?,
        };

        Ok(Self {
            handle: Arc::new(handle),
        })
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    pub(crate) async fn open_session(&self) -> Result<Channel<Msg>, SshError> {
        self.handle
            .channel_open_session()
            .await
            .map_err(|error| SshError::Channel(error.to_string()))
    }

    pub(crate) async fn open_direct(
        &self,
        host: &str,
        port: u16,
        originator: SocketAddr,
    ) -> Result<Channel<Msg>, SshError> {
        self.handle
            .channel_open_direct_tcpip(
                host,
                u32::from(port),
                originator.ip().to_string(),
                u32::from(originator.port()),
            )
            .await
            .map_err(|error| SshError::Channel(error.to_string()))
    }

    pub(crate) async fn close(&self) {
        let farewell = self
            .handle
            .disconnect(Disconnect::ByApplication, "", LANGUAGE_TAG);

        let _ = time::timeout(CLOSE_LIMIT, farewell).await;
    }
}

fn client_config(known_hosts: &KnownHosts, hop: &Target) -> Config {
    let mut config = Config {
        nodelay: true,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: KEEPALIVE_MISSES,
        ..Config::default()
    };

    let known = known_hosts.known_algorithms(hop.host(), hop.port());
    config.preferred.key = known_first(&known, &config.preferred.key).into();

    config
}

fn known_first(known: &[Algorithm], offered: &[Algorithm]) -> Vec<Algorithm> {
    let is_known =
        |algorithm: &&Algorithm| known.iter().any(|pinned| same_family(pinned, algorithm));

    let (first, rest): (Vec<&Algorithm>, Vec<&Algorithm>) = offered.iter().partition(is_known);

    first.into_iter().chain(rest).cloned().collect()
}

async fn dial(
    hop: &Target,
    config: &Arc<Config>,
    known_hosts: &Arc<KnownHosts>,
) -> Result<Handle<ClientHandler>, SshError> {
    let stream = TcpStream::connect((hop.host(), hop.port()))
        .await
        .map_err(|error| connect_error(hop, &error))?;
    stream.set_nodelay(true)?;

    handshake(hop, stream, config, known_hosts).await
}

async fn tunnel(
    previous: &Handle<ClientHandler>,
    hop: &Target,
    config: &Arc<Config>,
    known_hosts: &Arc<KnownHosts>,
) -> Result<Handle<ClientHandler>, SshError> {
    let channel = previous
        .channel_open_direct_tcpip(
            hop.host(),
            u32::from(hop.port()),
            ORIGINATOR_ADDRESS,
            ORIGINATOR_PORT,
        )
        .await
        .map_err(|error| connect_error(hop, &error))?;

    handshake(hop, channel.into_stream(), config, known_hosts).await
}

async fn handshake<S>(
    hop: &Target,
    stream: S,
    config: &Arc<Config>,
    known_hosts: &Arc<KnownHosts>,
) -> Result<Handle<ClientHandler>, SshError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let handler = ClientHandler::new(hop, Arc::clone(known_hosts));

    let mut handle = client::connect_stream(Arc::clone(config), stream, handler)
        .await
        .map_err(|error| match error {
            HandshakeError::Rejected(rejection) => rejection,
            HandshakeError::Protocol(protocol) => connect_error(hop, &protocol),
        })?;

    Authenticator::new(hop).authenticate(&mut handle).await?;

    Ok(handle)
}

fn connect_error(hop: &Target, error: &impl Display) -> SshError {
    SshError::Connect {
        host: hop.address(),
        reason: error.to_string(),
    }
}
