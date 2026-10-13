use std::ffi::OsStr;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use interprocess::os::windows::named_pipe::pipe_mode::Bytes;
use interprocess::os::windows::named_pipe::tokio::PipeStream;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::time;

const ERROR_PIPE_BUSY: i32 = 231;
const BUSY_RETRIES: u32 = 50;
const BUSY_PAUSE: Duration = Duration::from_millis(20);

pub(crate) enum Stream {
    Server(PipeStream<Bytes, Bytes>),
    Client(NamedPipeClient),
}

pub(crate) async fn connect(address: &OsStr) -> io::Result<Stream> {
    let mut attempts = 0;

    loop {
        match ClientOptions::new().open(address) {
            Ok(client) => return Ok(Stream::Client(client)),
            Err(error)
                if error.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < BUSY_RETRIES =>
            {
                attempts += 1;
                time::sleep(BUSY_PAUSE).await;
            }
            Err(error) => return Err(error),
        }
    }
}

impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Server(pipe) => Pin::new(pipe).poll_read(context, buffer),
            Self::Client(pipe) => Pin::new(pipe).poll_read(context, buffer),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Server(pipe) => Pin::new(pipe).poll_write(context, buffer),
            Self::Client(pipe) => Pin::new(pipe).poll_write(context, buffer),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Server(pipe) => Pin::new(pipe).poll_flush(context),
            Self::Client(pipe) => Pin::new(pipe).poll_flush(context),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Server(pipe) => Pin::new(pipe).poll_shutdown(context),
            Self::Client(pipe) => Pin::new(pipe).poll_shutdown(context),
        }
    }
}
