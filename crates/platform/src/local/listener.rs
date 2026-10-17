mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;

use native::NativeListener;
#[cfg(unix)]
use unix::SocketListener as OsListener;
#[cfg(windows)]
use windows::PipeServer as OsListener;

use super::LocalStream;
use crate::Endpoint;

pub struct LocalListener {
    inner: OsListener,
}

impl LocalListener {
    pub fn bind(endpoint: &Endpoint) -> io::Result<Self> {
        let inner = OsListener::bind(endpoint.address())?;

        Ok(Self { inner })
    }

    pub async fn accept(&mut self) -> io::Result<LocalStream> {
        let stream = self.inner.accept().await?;

        Ok(LocalStream::new(stream))
    }
}
