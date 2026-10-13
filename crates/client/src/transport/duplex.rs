use tokio::io::{AsyncRead, AsyncWrite};

pub(crate) trait Duplex: AsyncRead + AsyncWrite + Send + Unpin {}

impl<S: AsyncRead + AsyncWrite + Send + Unpin> Duplex for S {}
