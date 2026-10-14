use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use synt_kube::{Dialer, Dialing, Duplex};
use tokio::net::TcpStream;

pub struct TcpDialer {
    port: u16,
    dialed: Arc<AtomicUsize>,
}

impl TcpDialer {
    pub fn to_local_port(port: u16) -> Self {
        Self {
            port,
            dialed: Arc::default(),
        }
    }

    pub fn counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.dialed)
    }
}

impl Dialer for TcpDialer {
    fn dial(&self, _host: &str, _port: u16) -> Dialing {
        let port = self.port;
        self.dialed.fetch_add(1, Ordering::Relaxed);

        Box::pin(async move {
            let stream = TcpStream::connect(("127.0.0.1", port)).await?;

            Ok(Box::new(stream) as Box<dyn Duplex>)
        })
    }
}
