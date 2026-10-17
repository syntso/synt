use std::sync::Arc;

use k8s_openapi::api::core::v1::Pod;
use kube::api::Api;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use super::{Backend, ForwardTraffic};
use crate::KubeError;

const CONCURRENT_CONNECTIONS: usize = 64;
const BUFFER_BYTES: usize = 32 * 1024;

#[derive(Clone)]
pub(super) struct Relay {
    pods: Api<Pod>,
    backend: Arc<Backend>,
    traffic: Arc<ForwardTraffic>,
}

impl Relay {
    pub(super) fn new(pods: Api<Pod>, backend: Backend) -> Self {
        Self {
            pods,
            backend: Arc::new(backend),
            traffic: Arc::default(),
        }
    }

    pub(super) fn pod(&self) -> &str {
        &self.backend.pod
    }

    pub(super) fn port(&self) -> u16 {
        self.backend.port
    }

    pub(super) fn traffic(&self) -> Arc<ForwardTraffic> {
        Arc::clone(&self.traffic)
    }

    pub(super) async fn serve(self, listener: TcpListener) {
        let slots = Arc::new(Semaphore::new(CONCURRENT_CONNECTIONS));
        let mut connections = JoinSet::new();

        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((socket, _)) = accepted else {
                        continue;
                    };
                    let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
                        self.traffic.note(format!("more than {CONCURRENT_CONNECTIONS} connections at once"));
                        continue;
                    };

                    let relay = self.clone();
                    connections.spawn(async move {
                        relay.traffic.track(relay.carry(socket)).await;
                        drop(slot);
                    });
                }
                Some(_) = connections.join_next() => {}
            }
        }
    }

    async fn carry(&self, socket: TcpStream) -> Result<(), KubeError> {
        socket.set_nodelay(true)?;

        let port = self.port();
        let mut forwarder = self.pods.portforward(self.pod(), &[port]).await?;
        let stream = forwarder
            .take_stream(port)
            .ok_or(KubeError::NoStream(port))?;

        let (remote_read, remote_write) = io::split(stream);
        let (local_read, local_write) = socket.into_split();

        let sent = |bytes| self.traffic.add_sent(bytes);
        let received = |bytes| self.traffic.add_received(bytes);

        let outgoing = pump(local_read, remote_write, sent);
        let incoming = pump(remote_read, local_write, received);
        let carried = tokio::try_join!(outgoing, incoming);

        forwarder.abort();

        carried?;

        Ok(())
    }
}

async fn pump<R, W>(mut from: R, mut to: W, count: impl Fn(usize)) -> io::Result<()>
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
        count(read);
    }

    to.shutdown().await
}
