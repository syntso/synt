use std::future::Future;
use std::io;
use std::pin::Pin;

use tokio::io::{AsyncRead, AsyncWrite};

pub trait Duplex: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin + 'static> Duplex for T {}

pub type Dialing = Pin<Box<dyn Future<Output = io::Result<Box<dyn Duplex>>> + Send>>;

pub trait Dialer: Send + Sync + 'static {
    fn dial(&self, host: &str, port: u16) -> Dialing;
}
