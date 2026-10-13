use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::net;

use crate::pool::locked;

const REMEMBERED_FOR: Duration = Duration::from_secs(60);

type Endpoint = (String, u16);

#[derive(Default)]
pub(crate) struct Addresses {
    resolved: Mutex<HashMap<Endpoint, (Instant, Vec<SocketAddr>)>>,
}

impl Addresses {
    pub(crate) async fn lookup(&self, host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
        let endpoint = (host.to_owned(), port);

        if let Some(fresh) = self.remembered(&endpoint) {
            return Ok(fresh);
        }

        let found: Vec<SocketAddr> = net::lookup_host((host, port)).await?.collect();

        if found.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the name has no addresses",
            ));
        }

        locked(&self.resolved).insert(endpoint, (Instant::now(), found.clone()));

        Ok(found)
    }

    pub(crate) fn is_remembered(&self, host: &str, port: u16) -> bool {
        self.remembered(&(host.to_owned(), port)).is_some()
    }

    pub(crate) fn forget(&self, host: &str, port: u16) {
        locked(&self.resolved).remove(&(host.to_owned(), port));
    }

    fn remembered(&self, endpoint: &Endpoint) -> Option<Vec<SocketAddr>> {
        let mut resolved = locked(&self.resolved);

        let (at, addresses) = resolved.get(endpoint)?;

        if at.elapsed() < REMEMBERED_FOR {
            return Some(addresses.clone());
        }

        resolved.remove(endpoint);
        None
    }
}
