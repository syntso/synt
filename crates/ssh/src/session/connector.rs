use std::fmt::Display;
use std::sync::{Arc, OnceLock};

use russh::client::{self, Config, Handle};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use super::{Algorithms, ClientHandler, Connection, HandshakeError, client_config};
use crate::auth::{Authenticator, Credentials, Logins};
use crate::dial::{Addresses, race};
use crate::known_hosts::KnownHosts;
use crate::{KeyVault, SshError, Target};

#[derive(Default)]
pub(crate) struct Connector {
    addresses: Addresses,
    logins: Logins,
    vault: Option<Arc<dyn KeyVault>>,
}

impl Connector {
    pub(crate) fn with_vault(vault: Arc<dyn KeyVault>) -> Self {
        Self {
            vault: Some(vault),
            ..Self::default()
        }
    }

    pub(crate) async fn open(
        &self,
        hop: &Target,
        through: Option<&Connection>,
    ) -> Result<Connection, SshError> {
        let known_hosts = Arc::new(KnownHosts::load().await?);
        let config = Arc::new(client_config(&known_hosts, hop));
        let negotiated = Arc::new(OnceLock::new());

        let reaching = self.reach(hop, through, &config, &known_hosts, &negotiated);
        let gathering = Credentials::gather(hop, self.logins.accepted_key(hop));
        let (reached, credentials) = tokio::join!(reaching, gathering);
        let mut handle = reached?;

        let authenticator = Authenticator::new(hop, credentials, self.vault.clone());
        let accepted = authenticator.authenticate(&mut handle).await?;
        self.logins.remember(hop, accepted);

        Ok(Connection::new(handle, negotiated))
    }

    async fn reach(
        &self,
        hop: &Target,
        through: Option<&Connection>,
        config: &Arc<Config>,
        known_hosts: &Arc<KnownHosts>,
        negotiated: &Arc<OnceLock<Algorithms>>,
    ) -> Result<Handle<ClientHandler>, SshError> {
        let Some(previous) = through else {
            let stream = self.dial(hop).await?;
            return handshake(hop, stream, config, known_hosts, negotiated).await;
        };

        let channel = previous
            .open_hop(hop)
            .await
            .map_err(|error| connect_error(hop, &error))?;

        let stream = channel.into_stream();

        handshake(hop, stream, config, known_hosts, negotiated).await
    }

    async fn dial(&self, hop: &Target) -> Result<TcpStream, SshError> {
        let remembered = self.addresses.is_remembered(hop.host(), hop.port());

        match self.dial_once(hop).await {
            Ok(stream) => Ok(stream),
            Err(_) if remembered => self.dial_once(hop).await,
            Err(error) => Err(error),
        }
    }

    async fn dial_once(&self, hop: &Target) -> Result<TcpStream, SshError> {
        let addresses = self
            .addresses
            .lookup(hop.host(), hop.port())
            .await
            .map_err(|error| connect_error(hop, &error))?;

        let connected = race(&addresses).await;

        if connected.is_err() {
            self.addresses.forget(hop.host(), hop.port());
        }

        connected.map_err(|error| connect_error(hop, &error))
    }
}

async fn handshake<S>(
    hop: &Target,
    stream: S,
    config: &Arc<Config>,
    known_hosts: &Arc<KnownHosts>,
    negotiated: &Arc<OnceLock<Algorithms>>,
) -> Result<Handle<ClientHandler>, SshError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let handler = ClientHandler::new(hop, Arc::clone(known_hosts), Arc::clone(negotiated));

    client::connect_stream(Arc::clone(config), stream, handler)
        .await
        .map_err(|error| match error {
            HandshakeError::Rejected(rejection) => rejection,
            HandshakeError::Protocol(protocol) => connect_error(hop, &protocol),
        })
}

fn connect_error(hop: &Target, error: &impl Display) -> SshError {
    SshError::Connect {
        host: hop.address(),
        reason: error.to_string(),
    }
}
