mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::{Endpoint, Peer};
use native::NativeStream;
#[cfg(unix)]
pub(super) use tokio::net::UnixStream as OsStream;
#[cfg(windows)]
pub(super) use windows::Pipe as OsStream;

pub struct LocalStream {
    inner: OsStream,
}

impl LocalStream {
    pub(super) fn new(inner: OsStream) -> Self {
        Self { inner }
    }

    pub async fn connect(endpoint: &Endpoint) -> io::Result<Self> {
        let inner = OsStream::open(endpoint.address()).await?;

        Ok(Self { inner })
    }

    pub async fn peer(&self) -> io::Result<Peer> {
        self.inner.peer().await
    }
}

impl AsyncRead for LocalStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl AsyncWrite for LocalStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(context, buffer)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(context)
    }
}
