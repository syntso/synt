use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::Mutex;

use synt_platform::Lock;
use tokio::net;

use super::Resolved;
use crate::Destination;

#[derive(Default)]
pub(crate) struct Addresses {
    resolved: Mutex<HashMap<Destination, Resolved>>,
}

impl Addresses {
    pub(crate) async fn lookup(&self, destination: &Destination) -> io::Result<Vec<SocketAddr>> {
        if let Some(fresh) = self.remembered(destination) {
            return Ok(fresh);
        }

        let found: Vec<SocketAddr> = net::lookup_host((destination.host(), destination.port()))
            .await?
            .collect();

        if found.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the name has no addresses",
            ));
        }

        self.resolved
            .locked()
            .insert(destination.clone(), Resolved::now(found.clone()));

        Ok(found)
    }

    pub(crate) fn is_remembered(&self, destination: &Destination) -> bool {
        self.remembered(destination).is_some()
    }

    pub(crate) fn forget(&self, destination: &Destination) {
        self.resolved.locked().remove(destination);
    }

    fn remembered(&self, destination: &Destination) -> Option<Vec<SocketAddr>> {
        let mut resolved = self.resolved.locked();

        let fresh = resolved.get(destination)?.fresh();

        if fresh.is_none() {
            resolved.remove(destination);
        }

        fresh
    }
}
