use wasmtime::Store;

use super::{Exports, GuestCall};
use crate::bindings::v2::Extension;
use crate::bindings::v2::synt::extension::types;
use crate::documents::MergeSides;
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

    fn start(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<()> {
        let started = self.call_start(store)?;

        Ok(started.map_err(GuestError::from)?)
    }

    fn render(
        &self,
        store: &mut Store<Sandbox>,
        view: &str,
        target: Option<&str>,
        params: &str,
    ) -> wasmtime::Result<String> {
        let rendered = self.call_render(store, view, target, params)?;

        Ok(rendered.map_err(GuestError::from)?)
    }

    fn panel_message(
        &self,
        store: &mut Store<Sandbox>,
        panel: &str,
        message: &str,
    ) -> wasmtime::Result<String> {
        let reply = self.call_panel_message(store, panel, message)?;

        Ok(reply.map_err(GuestError::from)?)
    }

    fn asset(&self, store: &mut Store<Sandbox>, path: &str) -> wasmtime::Result<Option<Vec<u8>>> {
        self.call_asset(store, path)
    }

    fn on_event(
        &self,
        store: &mut Store<Sandbox>,
        kind: &str,
        payload: &str,
    ) -> wasmtime::Result<()> {
        let event = types::Event {
            kind: kind.to_owned(),
            payload: payload.to_owned(),
        };

        let heard = self.call_on_event(store, &event)?;

        Ok(heard.map_err(GuestError::from)?)
    }

    fn on_timer(&self, store: &mut Store<Sandbox>, name: &str) -> wasmtime::Result<()> {
        let woke = self.call_on_timer(store, name)?;

        Ok(woke.map_err(GuestError::from)?)
    }

    fn merge(
        &self,
        store: &mut Store<Sandbox>,
        kind: &str,
        sides: MergeSides<'_>,
    ) -> wasmtime::Result<Vec<u8>> {
        let merged = self.call_merge(store, kind, sides.base(), sides.ours(), sides.theirs())?;

        Ok(merged.map_err(GuestError::from)?)
    }
}
