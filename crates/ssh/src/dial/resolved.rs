use std::net::SocketAddr;
use std::time::{Duration, Instant};

const REMEMBERED_FOR: Duration = Duration::from_secs(60);

pub(super) struct Resolved {
    at: Instant,
    addresses: Vec<SocketAddr>,
}

impl Resolved {
    pub(super) fn now(addresses: Vec<SocketAddr>) -> Self {
        Self {
            at: Instant::now(),
            addresses,
        }
    }

    pub(super) fn fresh(&self) -> Option<Vec<SocketAddr>> {
        (self.at.elapsed() < REMEMBERED_FOR).then(|| self.addresses.clone())
    }
}
