use std::ffi::{OsStr, OsString};
use std::io;

use interprocess::os::windows::named_pipe::tokio::PipeListener;
use interprocess::os::windows::named_pipe::{PipeListenerOptions, pipe_mode};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

use super::Stream;

type Pipes = PipeListener<pipe_mode::Bytes, pipe_mode::Bytes>;

pub(crate) struct Listener {
    name: OsString,
    pipes: Pipes,
}

impl Listener {
    pub(crate) fn bind(address: &OsStr) -> io::Result<Self> {
        let first = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(address)?;

        Ok(Self {
            name: address.to_owned(),
            pipes: Self::pipes(address, first)?,
        })
    }

    pub(crate) async fn accept(&mut self) -> io::Result<Stream> {
        match self.pipes.accept().await {
            Ok(pipe) => Ok(Stream::Server(pipe)),
            Err(error) => {
                self.pipes = self.fresh_pipes()?;
                Err(error)
            }
        }
    }

    fn fresh_pipes(&self) -> io::Result<Pipes> {
        let instance = ServerOptions::new()
            .reject_remote_clients(true)
            .create(&self.name)?;

        Self::pipes(&self.name, instance)
    }

    fn pipes(address: &OsStr, instance: NamedPipeServer) -> io::Result<Pipes> {
        let options = PipeListenerOptions::new().path(address).to_owned()?;
        Ok(PipeListener::from_tokio_and_options(instance, options))
    }
}
