use std::io;
use std::sync::Arc;
use std::time::Duration;

use synt_kube::{Dialer, Dialing, Duplex};
use synt_ssh::{Destination, Lease};
use tokio::time;

const DIAL_LIMIT: Duration = Duration::from_secs(30);

pub(crate) struct SshDialer {
    lease: Arc<Lease>,
}

impl SshDialer {
    pub(crate) fn new(lease: Lease) -> Self {
        Self {
            lease: Arc::new(lease),
        }
    }
}

impl Dialer for SshDialer {
    fn dial(&self, host: &str, port: u16) -> Dialing {
        let lease = Arc::clone(&self.lease);
        let destination = Destination::new(host, port);

        Box::pin(async move {
            let destination = destination.map_err(io::Error::other)?;
            let opening = time::timeout(DIAL_LIMIT, lease.open_stream(&destination));

            let stream = opening
                .await
                .map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!(
                            "{destination} did not answer through ssh within {} s",
                            DIAL_LIMIT.as_secs()
                        ),
                    )
                })?
                .map_err(|error| {
                    io::Error::other(format!("through ssh to {destination}: {error}"))
                })?;

            Ok(Box::new(stream) as Box<dyn Duplex>)
        })
    }
}
