use super::Sandbox;
use crate::bindings::v1::ExtensionImports;
use crate::bindings::v1::synt::extension::types::{self, Error, Level};
use crate::runtime::Spend;

impl types::Host for Sandbox {}

impl ExtensionImports for Sandbox {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> wasmtime::Result<Result<String, Error>> {
        self.spend(Spend::Call, |imports| {
            imports.call(action, target, params).map_err(Error::from)
        })
    }

    fn storage_get(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        self.spend(Spend::Storage, |imports| imports.storage_get(key))
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.spend(Spend::Storage, |imports| {
            imports.storage_set(key, value).map_err(Error::from)
        })
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.spend(Spend::Free, |imports| imports.log(level.into(), message))
    }
}
