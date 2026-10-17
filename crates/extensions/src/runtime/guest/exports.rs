use wasmtime::Store;

use super::GuestCall;
use crate::documents::MergeSides;
use crate::interface::{DescribedAction, GuestError};
use crate::runtime::Sandbox;

pub(crate) trait Exports {
    fn describe(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<Vec<DescribedAction>>;

    fn run(&self, store: &mut Store<Sandbox>, call: &GuestCall) -> wasmtime::Result<String>;

    fn start(&self, _store: &mut Store<Sandbox>) -> wasmtime::Result<()> {
        Ok(())
    }

    fn render(
        &self,
        _store: &mut Store<Sandbox>,
        _view: &str,
        _target: Option<&str>,
        _params: &str,
    ) -> wasmtime::Result<String> {
        Err(GuestError::unsupported("views").into())
    }

    fn panel_message(
        &self,
        _store: &mut Store<Sandbox>,
        _panel: &str,
        _message: &str,
    ) -> wasmtime::Result<String> {
        Err(GuestError::unsupported("panels").into())
    }

    fn asset(&self, _store: &mut Store<Sandbox>, _path: &str) -> wasmtime::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn on_event(
        &self,
        _store: &mut Store<Sandbox>,
        _kind: &str,
        _payload: &str,
    ) -> wasmtime::Result<()> {
        Err(GuestError::unsupported("events").into())
    }

    fn on_timer(&self, _store: &mut Store<Sandbox>, _name: &str) -> wasmtime::Result<()> {
        Err(GuestError::unsupported("timers").into())
    }

    fn merge(
        &self,
        _store: &mut Store<Sandbox>,
        _kind: &str,
        _sides: MergeSides<'_>,
    ) -> wasmtime::Result<Vec<u8>> {
        Err(GuestError::unsupported("documents").into())
    }
}
