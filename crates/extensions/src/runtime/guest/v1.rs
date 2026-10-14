use synt_actions::Risk;
use wasmtime::Store;

use super::compiled::Template;
use super::{Answer, Exports, GuestCall};
use crate::bindings::v1::synt::extension::types;
use crate::bindings::v1::{Extension, ExtensionPre};
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
