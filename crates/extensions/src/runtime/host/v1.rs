use super::Sandbox;
use crate::bindings::v1::ExtensionImports;
use crate::bindings::v1::synt::extension::types::{self, Error, Level};
use crate::interface::{self, GuestError};

impl types::Host for Sandbox {}

impl ExtensionImports for Sandbox {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> wasmtime::Result<Result<String, Error>> {
        self.reach_out(|imports| imports.call(action, target, params).map_err(Error::from))
    }

    fn storage_get(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        self.keep(|imports| imports.storage_get(key))
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.storage_set(key, value).map_err(Error::from))
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.quietly(|imports| imports.log(level.into(), message));

        Ok(())
    }
}

impl From<GuestError> for Error {
    fn from(error: GuestError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}

impl From<Level> for interface::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Trace => Self::Trace,
            Level::Debug => Self::Debug,
            Level::Info => Self::Info,
            Level::Warn => Self::Warn,
            Level::Error => Self::Error,
        }
    }
}
