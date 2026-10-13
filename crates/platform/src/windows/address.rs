use std::env;
use std::ffi::OsString;
use std::io;
use std::path::Path;

pub(crate) fn address(_directory: &Path, name: &str) -> OsString {
    OsString::from(format!(r"\\.\pipe\{name}"))
}

pub(crate) fn default_address(name: &str) -> io::Result<OsString> {
    let user = env::var("USERNAME")
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "USERNAME is not set"))?;

    Ok(OsString::from(format!(r"\\.\pipe\{name}-{user}")))
}
