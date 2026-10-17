use wasmtime::Store;

use super::{Exports, GuestCall};
use crate::bindings::v1::Extension;
use crate::interface::{DescribedAction, GuestError};
use crate::runtime::Sandbox;

impl Exports for Extension {
    fn describe(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<Vec<DescribedAction>> {
        let described = self.call_describe(store)?;

        Ok(described.into_iter().map(DescribedAction::from).collect())
    }

    fn run(&self, store: &mut Store<Sandbox>, call: &GuestCall) -> wasmtime::Result<String> {
        let ran = self.call_run(store, call.action(), call.target(), call.params())?;

        Ok(ran.map_err(GuestError::from)?)
    }
}
