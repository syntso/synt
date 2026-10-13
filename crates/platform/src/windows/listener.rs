use std::ffi::{OsStr, OsString};
use std::io;
use std::mem;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

use super::Stream;

pub(crate) struct Listener {
    name: OsString,
    waiting: NamedPipeServer,
}

impl Listener {
    pub(crate) fn bind(address: &OsStr) -> io::Result<Self> {
        let waiting = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(address)?;

        Ok(Self {
            name: address.to_owned(),
            waiting,
        })
    }

    pub(crate) async fn accept(&mut self) -> io::Result<Stream> {
        let connected = self.waiting.connect().await;

        let next = self.fresh_instance()?;
        let finished = mem::replace(&mut self.waiting, next);

        connected?;
        Ok(Stream::Server(finished))
    }

    fn fresh_instance(&self) -> io::Result<NamedPipeServer> {
        ServerOptions::new()
            .reject_remote_clients(true)
            .create(&self.name)
    }
}
