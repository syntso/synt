use std::sync::Arc;

use russh::client::Handle;
use tokio::net::TcpStream;

use super::{ClientHandler, Connection, Handshake};
use crate::authenticator::{Authenticator, Credentials, Logins};
use crate::dial::{Addresses, Race};
use crate::directory::SshDirectory;
use crate::known_hosts::KnownHosts;
use crate::{KeyVault, SshError, Target};

pub(crate) struct Connector {
    addresses: Addresses,
    logins: Logins,
    vault: Option<Arc<dyn KeyVault>>,
    directory: SshDirectory,
}

impl Connector {
    pub(crate) fn new(vault: Option<Arc<dyn KeyVault>>, directory: SshDirectory) -> Self {
        Self {
            addresses: Addresses::default(),
            logins: Logins::default(),
            vault,
            directory,
        }
    }

    pub(crate) fn directory(&self) -> &SshDirectory {
        &self.directory
    }

    pub(crate) async fn open(
        &self,
        hop: &Target,
        through: Option<&Connection>,
    ) -> Result<Connection, SshError> {
        let known_hosts = KnownHosts::load(&self.directory).await?;
        let handshake = Handshake::new(hop, known_hosts);

        let reaching = self.reach(hop, through, &handshake);
        let accepted_before = self.logins.accepted_key(hop);
        let gathering = Credentials::gather(hop, accepted_before, &self.directory);
        let (reached, credentials) = tokio::join!(reaching, gathering);
        let mut handle = reached?;

        let authenticator = Authenticator::new(hop, credentials, self.vault.clone());
        let accepted = authenticator.authenticate(&mut handle).await?;
        self.logins.remember(hop, accepted);

        Ok(Connection::new(handle, handshake.negotiated()))
    }

    async fn reach(
        &self,
        hop: &Target,
        through: Option<&Connection>,
        handshake: &Handshake,
    ) -> Result<Handle<ClientHandler>, SshError> {
        let Some(previous) = through else {
            let stream = self.dial(hop).await?;
            return handshake.over(stream).await;
        };

        let channel = previous
            .open_hop(hop)
            .await
            .map_err(|error| SshError::connect(hop.address(), error))?;

        handshake.over(channel.into_stream()).await
    }

    async fn dial(&self, hop: &Target) -> Result<TcpStream, SshError> {
        let destination = hop.destination()?;
        let remembered = self.addresses.is_remembered(&destination);

        match self.dial_once(hop).await {
            Ok(stream) => Ok(stream),
            Err(_) if remembered => self.dial_once(hop).await,
            Err(error) => Err(error),
        }
    }

    async fn dial_once(&self, hop: &Target) -> Result<TcpStream, SshError> {
        let destination = hop.destination()?;

        let addresses = self
            .addresses
            .lookup(&destination)
            .await
            .map_err(|error| SshError::connect(hop.address(), error))?;

        let connected = Race::between(&addresses).winner().await;

        if connected.is_err() {
            self.addresses.forget(&destination);
        }

        connected.map_err(|error| SshError::connect(hop.address(), error))
    }
}
