mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

use native::NativeAddress;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

const NAME: &str = "synt-core";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Endpoint {
    address: OsString,
}

impl Endpoint {
    pub fn new(address: impl Into<OsString>) -> Self {
        Self {
            address: address.into(),
        }
    }

    pub fn for_user() -> io::Result<Self> {
        let address = Os::default_address(NAME)?;

        Ok(Self { address })
    }

    pub fn named(directory: &Path, name: &str) -> Self {
        Self {
            address: Os::address(directory, name),
        }
    }

    pub fn address(&self) -> &OsStr {
        &self.address
    }

    pub fn absolute_address(&self) -> OsString {
        Os::absolute_address(&self.address)
    }
}
