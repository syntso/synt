use std::ffi::OsStr;
use std::io;

pub(crate) type Stream = tokio::net::UnixStream;

pub(crate) async fn connect(address: &OsStr) -> io::Result<Stream> {
    Stream::connect(address).await
}
