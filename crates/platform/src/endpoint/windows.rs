use std::env;
use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

use super::NativeAddress;

pub(super) struct Windows;

impl NativeAddress for Windows {
    fn address(_directory: &Path, name: &str) -> OsString {
        OsString::from(format!(r"\\.\pipe\{name}"))
    }

    fn default_address(name: &str) -> io::Result<OsString> {
        let user = env::var("USERNAME")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "USERNAME is not set"))?;

        Ok(OsString::from(format!(r"\\.\pipe\{name}-{user}")))
    }

    fn absolute_address(address: &OsStr) -> OsString {
        address.to_owned()
    }
}
