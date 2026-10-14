use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;

use russh::Channel;
use russh::client::Msg;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::runtime;

use super::{ConnectionState, Slot};
use crate::session::Connection;
use crate::{Algorithms, Destination, SshError, Target};

type Connecting<'a> = Pin<Box<dyn Future<Output = Result<Connection, SshError>> + Send + 'a>>;

pub struct Lease {
    slot: Arc<Slot>,
}

impl Lease {
    pub(crate) fn new(slot: Arc<Slot>) -> Self {
        slot.join();

        Self { slot }
    }

    pub fn target(&self) -> &Target {
        self.slot.target()
    }

    pub fn state(&self) -> ConnectionState {
        self.slot.status().state
    }

    pub fn algorithms(&self) -> Option<Algorithms> {
        self.slot.algorithms()
    }

    pub async fn open_stream(
        &self,
        destination: &Destination,
    ) -> Result<impl AsyncRead + AsyncWrite + Send + Unpin + 'static, SshError> {
        let originator = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let channel = self
            .open_direct(destination.host(), destination.port(), originator)
            .await?;

        Ok(channel.into_stream())
    }

    pub(crate) fn connection(&self) -> Connecting<'_> {
        Box::pin(self.slot.connection())
    }

    pub(crate) async fn keep_spare_session(&self) -> Result<(), SshError> {
        self.connection().await?.keep_spare_session().await;

        Ok(())
    }

    pub(crate) async fn open_session(&self) -> Result<Channel<Msg>, SshError> {
        let connection = self.connection().await?;

        match connection.open_session().await {
            Ok(channel) => Ok(channel),
            Err(_) if connection.is_closed() => self.connection().await?.open_session().await,
            Err(error) => Err(error),
        }
    }

    pub(crate) async fn open_direct(
        &self,
        host: &str,
        port: u16,
        originator: SocketAddr,
    ) -> Result<Channel<Msg>, SshError> {
        let connection = self.connection().await?;

        match connection.open_direct(host, port, originator).await {
            Ok(channel) => Ok(channel),
            Err(_) if connection.is_closed() => {
                let renewed = self.connection().await?;
                renewed.open_direct(host, port, originator).await
            }
            Err(error) => Err(error),
        }
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let Some(release) = self.slot.leave() else {
            return;
        };
        let Ok(runtime) = runtime::Handle::try_current() else {
            return;
        };

        let slot = Arc::clone(&self.slot);
        runtime.spawn(slot.expire(release));
    }
}

impl Debug for Lease {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Lease")
            .field("target", &self.target().label())
            .field("state", &self.state())
            .finish()
    }
}
