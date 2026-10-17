use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

pub(super) trait NativeAddress {
    fn address(directory: &Path, name: &str) -> OsString;

    fn default_address(name: &str) -> io::Result<OsString>;

    fn absolute_address(address: &OsStr) -> OsString;
}
