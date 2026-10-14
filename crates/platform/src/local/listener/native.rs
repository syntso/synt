use std::ffi::OsStr;
use std::io;

use crate::local::stream::OsStream;

pub(super) trait NativeListener: Sized {
    fn bind(address: &OsStr) -> io::Result<Self>;

    async fn accept(&mut self) -> io::Result<OsStream>;
}
