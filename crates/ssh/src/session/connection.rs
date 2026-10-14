use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use russh::client::{Handle, Msg};
use russh::{Channel, Disconnect};
use tokio::time;

use super::{Algorithms, ClientHandler, Spare};
use crate::{SshError, Target};

const CLOSE_LIMIT: Duration = Duration::from_secs(5);
const ORIGINATOR_ADDRESS: &str = "127.0.0.1";
const ORIGINATOR_PORT: u32 = 0;
const LANGUAGE_TAG: &str = "en";

#[derive(Clone)]
pub(crate) struct Connection {
    handle: Arc<Handle<ClientHandler>>,
    spare: Arc<Spare>,
    negotiated: Arc<OnceLock<Algorithms>>,
}

impl Connection {
    pub(crate) fn new(
        handle: Handle<ClientHandler>,
        negotiated: Arc<OnceLock<Algorithms>>,
    ) -> Self {
        Self {
            handle: Arc::new(handle),
            spare: Arc::new(Spare::default()),
            negotiated,
        }
    }

    pub(crate) fn algorithms(&self) -> Option<Algorithms> {
        self.negotiated.get().cloned()
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.handle.is_closed()
    }

    pub(crate) async fn keep_spare_session(&self) {
        self.spare.fill_now(&self.handle).await;
    }

    pub(crate) async fn open_session(&self) -> Result<Channel<Msg>, SshError> {
        let ready = self.spare.take();
        self.spare.fill(&self.handle);

        if let Some(channel) = ready {
            return Ok(channel);
        }

        let channel = self.handle.channel_open_session().await?;

        Ok(channel)
    }

    pub(crate) async fn open_direct(
        &self,
        host: &str,
        port: u16,
        originator: SocketAddr,
    ) -> Result<Channel<Msg>, SshError> {
        let channel = self
            .handle
            .channel_open_direct_tcpip(
                host,
                u32::from(port),
                originator.ip().to_string(),
                u32::from(originator.port()),
            )
            .await?;

        Ok(channel)
    }

    pub(crate) async fn open_hop(&self, hop: &Target) -> Result<Channel<Msg>, russh::Error> {
        self.handle
            .channel_open_direct_tcpip(
                hop.host(),
                u32::from(hop.port()),
                ORIGINATOR_ADDRESS,
                ORIGINATOR_PORT,
            )
            .await
    }

    pub(crate) async fn close(&self) {
        let farewell = self
            .handle
            .disconnect(Disconnect::ByApplication, "", LANGUAGE_TAG);

        let _ = time::timeout(CLOSE_LIMIT, farewell).await;
    }
}
