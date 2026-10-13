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

pub(super) async fn serve(
    listener: TcpListener,
    lease: Arc<Lease>,
    destination: Destination,
    traffic: Arc<Traffic>,
) {
    let mut forwards = JoinSet::new();
    let slots = Arc::new(Semaphore::new(CONCURRENT_FORWARDS));

    loop {
        tokio::select! {
            accepted = listener.accept() => match accepted {
                Ok((socket, peer)) => {
                    let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
                        drop(socket);
                        traffic.note(&SshError::TunnelBusy { limit: CONCURRENT_FORWARDS });
                        continue;
                    };

                    let lease = Arc::clone(&lease);
                    let destination = destination.clone();
                    let traffic = Arc::clone(&traffic);

                    forwards.spawn(forward(socket, peer, lease, destination, traffic, slot));
                }
                Err(error) => {
                    traffic.note(&SshError::from(error));
                    time::sleep(ACCEPT_PAUSE).await;
                }
            },
            Some(_) = forwards.join_next() => {}
        }
    }
}

async fn forward(
    socket: TcpStream,
    peer: SocketAddr,
    lease: Arc<Lease>,
    destination: Destination,
    traffic: Arc<Traffic>,
    _slot: OwnedSemaphorePermit,
) {
    traffic.begin();

    let piped = pipe(socket, peer, &lease, &destination, &traffic).await;

    traffic.end(piped);
}

async fn pipe(
    socket: TcpStream,
    peer: SocketAddr,
    lease: &Lease,
    destination: &Destination,
    traffic: &Traffic,
) -> Result<(), SshError> {
    socket.set_nodelay(true)?;

    let channel = lease
        .open_direct(destination.host(), destination.port(), peer)
        .await?;

    let (remote_read, remote_write) = io::split(channel.into_stream());
    let (local_read, local_write) = socket.into_split();

    let outgoing = pump(local_read, remote_write, traffic, Traffic::add_sent);
    let incoming = pump(remote_read, local_write, traffic, Traffic::add_received);

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
