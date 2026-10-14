use std::fmt::{self, Debug, Formatter};
use std::net::Ipv4Addr;
use std::sync::Arc;

use k8s_openapi::api::core::v1::Pod;
use kube::api::Api;
use tokio::io::{self, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::{JoinHandle, JoinSet};

use super::{ForwardStatus, ForwardTraffic};
use crate::KubeError;

const CONCURRENT_CONNECTIONS: usize = 64;
const BUFFER_BYTES: usize = 32 * 1024;

pub struct PortForward {
    target: String,
    namespace: String,
    pod: String,
    remote_port: u16,
    local_port: u16,
    traffic: Arc<ForwardTraffic>,
    serving: JoinHandle<()>,
}

impl PortForward {
    pub(crate) async fn start(
        pods: Api<Pod>,
        target: String,
        (namespace, pod): (String, String),
        remote_port: u16,
        local_port: Option<u16>,
    ) -> Result<Self, KubeError> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, local_port.unwrap_or(0)))
            .await
            .map_err(|error| {
                KubeError::Io(format!(
                    "cannot listen on 127.0.0.1:{}: {error}",
                    local_port.unwrap_or(0)
                ))
            })?;
        let local_port = listener.local_addr()?.port();
        let traffic = Arc::new(ForwardTraffic::default());

        let serving = tokio::spawn(serve(
            listener,
            pods,
            pod.clone(),
            remote_port,
            Arc::clone(&traffic),
        ));

        Ok(Self {
            target,
            namespace,
            pod,
            remote_port,
            local_port,
            traffic,
            serving,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn status(&self) -> ForwardStatus {
        ForwardStatus {
            target: self.target.clone(),
            namespace: self.namespace.clone(),
            pod: self.pod.clone(),
            remote_port: self.remote_port,
            local_port: self.local_port,
            connections: self.traffic.connections(),
            active: self.traffic.active(),
            sent_bytes: self.traffic.sent(),
            received_bytes: self.traffic.received(),
            last_error: self.traffic.last_error(),
        }
    }

    pub async fn close(mut self) -> ForwardStatus {
        self.serving.abort();
        let _stopped = (&mut self.serving).await;

        self.status()
    }
}

impl Drop for PortForward {
    fn drop(&mut self) {
        self.serving.abort();
    }
}

impl Debug for PortForward {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PortForward")
            .field("target", &self.target)
            .field("local_port", &self.local_port)
            .finish_non_exhaustive()
    }
}

async fn serve(
    listener: TcpListener,
    pods: Api<Pod>,
    pod: String,
    port: u16,
    traffic: Arc<ForwardTraffic>,
) {
    let slots = Arc::new(Semaphore::new(CONCURRENT_CONNECTIONS));
    let mut connections = JoinSet::new();

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((socket, _)) = accepted else {
                    continue;
                };
                let Ok(slot) = Arc::clone(&slots).try_acquire_owned() else {
                    traffic.note(format!("more than {CONCURRENT_CONNECTIONS} connections at once"));
                    continue;
                };

                let (pods, pod, traffic) = (pods.clone(), pod.clone(), Arc::clone(&traffic));
                connections.spawn(async move {
                    traffic.begin();
                    traffic.end(carry(socket, &pods, &pod, port, &traffic).await);
                    drop(slot);
                });
            }
            Some(_) = connections.join_next() => {}
        }
    }
}

async fn carry(
    socket: TcpStream,
    pods: &Api<Pod>,
    pod: &str,
    port: u16,
    traffic: &ForwardTraffic,
) -> Result<(), String> {
    socket
        .set_nodelay(true)
        .map_err(|error| error.to_string())?;

    let mut forwarder = pods
        .portforward(pod, &[port])
        .await
        .map_err(|error| KubeError::from(error).to_string())?;
    let stream = forwarder
        .take_stream(port)
        .ok_or_else(|| format!("the cluster opened no stream for port {port}"))?;

    let (remote_read, remote_write) = io::split(stream);
    let (local_read, local_write) = socket.into_split();

    let outgoing = pump(local_read, remote_write, traffic, ForwardTraffic::add_sent);
    let incoming = pump(
        remote_read,
        local_write,
        traffic,
        ForwardTraffic::add_received,
    );
    let carried = tokio::try_join!(outgoing, incoming).map_err(|error| error.to_string());

    forwarder.abort();

    carried.map(|_| ())
}

async fn pump<R, W>(
    mut from: R,
    mut to: W,
    traffic: &ForwardTraffic,
    count: fn(&ForwardTraffic, usize),
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
