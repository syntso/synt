use synt_actions::Risk;
use wasmtime::Store;

use super::compiled::Template;
use super::{Answer, Exports, GuestCall};
use crate::bindings::v2::synt::extension::types;
use crate::bindings::v2::{Extension, ExtensionPre};
use crate::interface::{DescribedAction, GuestError};
use crate::runtime::Sandbox;

impl Template for ExtensionPre<Sandbox> {
    fn instance(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<Box<dyn Exports>> {
        Ok(Box::new(self.instantiate(store)?))
    }
}

impl Exports for Extension {
    fn describe(&self, store: &mut Store<Sandbox>) -> wasmtime::Result<Vec<DescribedAction>> {
        let described = self.call_describe(store)?;

        Ok(described.into_iter().map(DescribedAction::from).collect())
    }

    fn run(&self, store: &mut Store<Sandbox>, call: &GuestCall) -> Answer<String> {
        let ran = self.call_run(store, &call.action, call.target.as_deref(), &call.params)?;

        Ok(ran.map_err(GuestError::from))
    }

    fn start(&self, store: &mut Store<Sandbox>) -> Answer<()> {
        let started = self.call_start(store)?;

        Ok(started.map_err(GuestError::from))
    }

    fn render(
        &self,
        store: &mut Store<Sandbox>,
        view: &str,
        target: Option<&str>,
        params: &str,
    ) -> Answer<String> {
        let rendered = self.call_render(store, view, target, params)?;

        Ok(rendered.map_err(GuestError::from))
    }

    fn panel_message(
        &self,
        store: &mut Store<Sandbox>,
        panel: &str,
        message: &str,
    ) -> Answer<String> {
        let reply = self.call_panel_message(store, panel, message)?;

        Ok(reply.map_err(GuestError::from))
    }

    fn asset(&self, store: &mut Store<Sandbox>, path: &str) -> wasmtime::Result<Option<Vec<u8>>> {
        self.call_asset(store, path)
    }

    fn on_event(&self, store: &mut Store<Sandbox>, kind: &str, payload: &str) -> Answer<()> {
        let event = types::Event {
            kind: kind.to_owned(),
            payload: payload.to_owned(),
        };

        let heard = self.call_on_event(store, &event)?;

        Ok(heard.map_err(GuestError::from))
    }

    fn on_timer(&self, store: &mut Store<Sandbox>, name: &str) -> Answer<()> {
        let woke = self.call_on_timer(store, name)?;

        Ok(woke.map_err(GuestError::from))
    }

    fn merge(
        &self,
        store: &mut Store<Sandbox>,
        kind: &str,
        sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Answer<Vec<u8>> {
        let (base, ours, theirs) = sides;

        let merged = self.call_merge(store, kind, base, ours, theirs)?;

        Ok(merged.map_err(GuestError::from))
    }
}

impl From<types::Action> for DescribedAction {
    fn from(action: types::Action) -> Self {
        Self {
            id: action.id,
            title: action.title,
            risk: action.risk.into(),
            takes_target: action.takes_target,
            params_schema: action.params_schema,
        }
    }
}

impl From<types::Risk> for Risk {
    fn from(risk: types::Risk) -> Self {
        match risk {
            types::Risk::Read => Self::Read,
            types::Risk::Write => Self::Write,
            types::Risk::Privileged => Self::Privileged,
        }
    }
}

impl From<types::Error> for GuestError {
    fn from(error: types::Error) -> Self {
        Self::new(error.code, error.message)
    }
}
