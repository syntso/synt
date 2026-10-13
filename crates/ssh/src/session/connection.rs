use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use russh::Disconnect;
use russh::client::{self, Config, Handle};
use russh::keys::Algorithm;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time;

use super::{ClientHandler, HandshakeError};
use crate::auth::Authenticator;
use crate::known_hosts::{KnownHosts, same_family};
use crate::{SshError, Target};

const CLOSE_LIMIT: Duration = Duration::from_secs(5);
const ORIGINATOR_ADDRESS: &str = "127.0.0.1";
const ORIGINATOR_PORT: u32 = 0;
const LANGUAGE_TAG: &str = "en";

pub(crate) struct Connection {
    hops: Vec<Handle<ClientHandler>>,
}

impl Connection {
    pub(crate) async fn open(target: &Target) -> Result<Self, SshError> {
        let known_hosts = Arc::new(KnownHosts::load().await?);
        let mut hops: Vec<Handle<ClientHandler>> = Vec::new();

        for hop in target.hops() {
            let config = Arc::new(client_config(&known_hosts, hop));
            let handle = match hops.last() {
                None => dial(hop, &config, &known_hosts).await?,
                Some(previous) => tunnel(previous, hop, &config, &known_hosts).await?,
            };
            hops.push(handle);
        }

        Ok(Self { hops })
    }

    pub(crate) fn target(&self) -> Result<&Handle<ClientHandler>, SshError> {
        self.hops
            .last()
            .ok_or_else(|| SshError::Channel("the connection has no hops".to_owned()))
    }

    pub(crate) async fn close(self) {
        for handle in self.hops.iter().rev() {
            let farewell = handle.disconnect(Disconnect::ByApplication, "", LANGUAGE_TAG);
            let _ = time::timeout(CLOSE_LIMIT, farewell).await;
        }
    }
}

fn client_config(known_hosts: &KnownHosts, hop: &Target) -> Config {
    let mut config = Config {
        nodelay: true,
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
