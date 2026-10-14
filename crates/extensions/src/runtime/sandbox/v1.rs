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
        self.reach_out(|imports| imports.call(action, target, params).map_err(error))
    }

    fn storage_get(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        self.keep(|imports| imports.storage_get(key))
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.keep(|imports| imports.storage_set(key, value).map_err(error))
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.quietly(|imports| imports.log(level_of(level), message));

        Ok(())
    }
}

fn error(error: GuestError) -> Error {
    Error {
        code: error.code,
        message: error.message,
    }
}

fn level_of(level: Level) -> interface::Level {
    match level {
        Level::Trace => interface::Level::Trace,
        Level::Debug => interface::Level::Debug,
        Level::Info => interface::Level::Info,
        Level::Warn => interface::Level::Warn,
        Level::Error => interface::Level::Error,
    }
}
