use std::ffi::{OsStr, OsString};
use std::io;

use interprocess::os::windows::named_pipe::tokio::PipeListener;
use interprocess::os::windows::named_pipe::{PipeListenerOptions, pipe_mode};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio_util::either::Either;

use super::NativeListener;
use crate::local::stream::OsStream;

type Pipes = PipeListener<pipe_mode::Bytes, pipe_mode::Bytes>;

pub(super) struct PipeServer {
    name: OsString,
    pipes: Pipes,
}

impl PipeServer {
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

impl NativeListener for PipeServer {
    fn bind(address: &OsStr) -> io::Result<Self> {
        let first = ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(address)?;

        Ok(Self {
            name: address.to_owned(),
            pipes: Self::pipes(address, first)?,
        })
    }

    async fn accept(&mut self) -> io::Result<OsStream> {
        match self.pipes.accept().await {
            Ok(pipe) => Ok(Either::Left(pipe)),
            Err(error) => {
                self.pipes = self.fresh_pipes()?;
                Err(error)
            }
        }
    }
}
