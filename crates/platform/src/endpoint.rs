use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

use crate::native;

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
        native::default_address(NAME).map(|address| Self { address })
    }

    pub fn named(directory: &Path, name: &str) -> Self {
        Self {
            address: native::address(directory, name),
        }
    }

    pub fn address(&self) -> &OsStr {
        &self.address
    }
}
