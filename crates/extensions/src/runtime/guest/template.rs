use wasmtime::Store;

use super::Exports;
use crate::runtime::Sandbox;

pub(crate) trait Template: Send + Sync {
    fn instance(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<Box<dyn Exports>>;
}
