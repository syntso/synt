use std::ffi::OsStr;
use std::io;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::Peer;

pub(in crate::local) trait NativeStream:
    AsyncRead + AsyncWrite + Unpin + Sized
{
    async fn open(address: &OsStr) -> io::Result<Self>;

    async fn peer(&self) -> io::Result<Peer>;
}
