use wasmtime::Store;

use super::GuestCall;
use crate::interface::{DescribedAction, GuestError};
use crate::runtime::Sandbox;

pub(crate) type Answer<T> = wasmtime::Result<Result<T, GuestError>>;

pub(crate) trait Exports {
    fn describe(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<Vec<DescribedAction>>;

    fn run(&self, store: &mut Store<Sandbox>, call: &GuestCall) -> Answer<String>;

    fn start(&self, _store: &mut Store<Sandbox>) -> Answer<()> {
        Ok(Ok(()))
    }

    fn render(
        &self,
        _store: &mut Store<Sandbox>,
        _view: &str,
        _target: Option<&str>,
        _params: &str,
    ) -> Answer<String> {
        Ok(Err(GuestError::unsupported("views")))
    }

    fn panel_message(
        &self,
        _store: &mut Store<Sandbox>,
        _panel: &str,
        _message: &str,
    ) -> Answer<String> {
        Ok(Err(GuestError::unsupported("panels")))
    }

    fn asset(&self, _store: &mut Store<Sandbox>, _path: &str) -> wasmtime::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn on_event(&self, _store: &mut Store<Sandbox>, _kind: &str, _payload: &str) -> Answer<()> {
        Ok(Err(GuestError::unsupported("events")))
    }

    fn on_timer(&self, _store: &mut Store<Sandbox>, _name: &str) -> Answer<()> {
        Ok(Err(GuestError::unsupported("timers")))
    }

    fn merge(
        &self,
        _store: &mut Store<Sandbox>,
        _kind: &str,
        _sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Answer<Vec<u8>> {
        Ok(Err(GuestError::unsupported("documents")))
    }
}
