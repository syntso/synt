use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;
use tokio::time;

use super::{Destination, Traffic};
use crate::{Lease, SshError};

const BUFFER_BYTES: usize = 32 * 1024;
const ACCEPT_PAUSE: Duration = Duration::from_millis(100);
const CONCURRENT_FORWARDS: usize = 64;

#[derive(Clone)]
pub(super) struct Forwarder {
    lease: Arc<Lease>,
    destination: Destination,
    traffic: Arc<Traffic>,
}

impl Forwarder {
    pub(super) fn new(lease: Arc<Lease>, destination: Destination, traffic: Arc<Traffic>) -> Self {
        Self {
            lease,
            destination,
            traffic,
        }
    }

    pub(super) async fn serve(self, listener: TcpListener) {
        let mut forwards = JoinSet::new();
        let slots = Arc::new(Semaphore::new(CONCURRENT_FORWARDS));

        loop {
            tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok((socket, peer)) => {
                        let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
                            drop(socket);
                            self.traffic.note(&SshError::TunnelBusy { limit: CONCURRENT_FORWARDS });
                            continue;
                        };

                        forwards.spawn(self.clone().forward(socket, peer, slot));
                    }
                    Err(error) => {
                        self.traffic.note(&SshError::from(error));
                        time::sleep(ACCEPT_PAUSE).await;
                    }
                },
                Some(_) = forwards.join_next() => {}
            }
        }
    }

    async fn forward(self, socket: TcpStream, peer: SocketAddr, _slot: OwnedSemaphorePermit) {
        self.traffic.begin();

        let piped = self.pipe(socket, peer).await;

        self.traffic.end(piped);
    }

    async fn pipe(&self, socket: TcpStream, peer: SocketAddr) -> Result<(), SshError> {
        socket.set_nodelay(true)?;

        let channel = self.lease.open_direct(&self.destination, peer).await?;

        let (remote_read, remote_write) = io::split(channel.into_stream());
        let (local_read, local_write) = socket.into_split();

        let outgoing = Self::pump(local_read, remote_write, &self.traffic, Traffic::add_sent);
        let incoming = Self::pump(
            remote_read,
            local_write,
            &self.traffic,
            Traffic::add_received,
        );

        tokio::try_join!(outgoing, incoming)?;

        Ok(())
    }

    async fn pump<R, W>(
        mut from: R,
        mut to: W,
        traffic: &Traffic,
        count: fn(&Traffic, usize),
    ) -> io::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buffer = vec![0; BUFFER_BYTES];

        loop {
            let read = from.read(&mut buffer).await?;

            if read == 0 {
                break;
            }

            to.write_all(&buffer[..read]).await?;
            count(traffic, read);
        }

        to.shutdown().await
    }
}
