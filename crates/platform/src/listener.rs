use std::io;

use crate::{Endpoint, LocalStream, native};

pub struct LocalListener {
    inner: native::Listener,
}

impl LocalListener {
    pub fn bind(endpoint: &Endpoint) -> io::Result<Self> {
        let inner = native::Listener::bind(endpoint.address())?;
        Ok(Self { inner })
    }

    pub async fn accept(&mut self) -> io::Result<LocalStream> {
        let stream = self.inner.accept().await?;
        Ok(LocalStream::from_inner(stream))
    }
}
