use synt_actions::Risk;
use wasmtime::Store;

use super::{GuestCall, Sandbox};
use crate::bindings::v1::synt::extension::types as types_v1;
use crate::bindings::v2::synt::extension::types as types_v2;
use crate::bindings::{v1, v2};
use crate::interface::{DescribedAction, GuestError};

type Answer<T> = wasmtime::Result<Result<T, GuestError>>;

pub(crate) enum Instance {
    V1(v1::Extension),
    V2(Box<v2::Extension>),
}

impl Instance {
    pub(crate) fn describe(
        &self,
        store: &mut Store<Sandbox>,
    ) -> wasmtime::Result<Vec<DescribedAction>> {
        match self {
            Self::V1(guest) => Ok(guest
                .call_describe(store)?
                .into_iter()
                .map(described_v1)
                .collect()),
            Self::V2(guest) => Ok(guest
                .call_describe(store)?
                .into_iter()
                .map(described_v2)
                .collect()),
        }
    }

    pub(crate) fn run(&self, store: &mut Store<Sandbox>, call: &GuestCall) -> Answer<String> {
        let target = call.target.as_deref();

        match self {
            Self::V1(guest) => guest
                .call_run(store, &call.action, target, &call.params)
                .map(|ran| ran.map_err(error_v1)),
            Self::V2(guest) => guest
                .call_run(store, &call.action, target, &call.params)
                .map(|ran| ran.map_err(error_v2)),
        }
    }

    pub(crate) fn start(&self, store: &mut Store<Sandbox>) -> Answer<()> {
        match self {
            Self::V1(_) => Ok(Ok(())),
            Self::V2(guest) => guest
                .call_start(store)
                .map(|started| started.map_err(error_v2)),
        }
    }

    pub(crate) fn render(
        &self,
        store: &mut Store<Sandbox>,
        view: &str,
        target: Option<&str>,
        params: &str,
    ) -> Answer<String> {
        let Self::V2(guest) = self else {
            return Ok(Err(GuestError::unsupported("views")));
        };

        guest
            .call_render(store, view, target, params)
            .map(|rendered| rendered.map_err(error_v2))
    }

    pub(crate) fn panel_message(
        &self,
        store: &mut Store<Sandbox>,
        panel: &str,
        message: &str,
    ) -> Answer<String> {
        let Self::V2(guest) = self else {
            return Ok(Err(GuestError::unsupported("panels")));
        };

        guest
            .call_panel_message(store, panel, message)
            .map(|reply| reply.map_err(error_v2))
    }

    pub(crate) fn asset(
        &self,
        store: &mut Store<Sandbox>,
        path: &str,
    ) -> wasmtime::Result<Option<Vec<u8>>> {
        match self {
            Self::V1(_) => Ok(None),
            Self::V2(guest) => guest.call_asset(store, path),
        }
    }

    pub(crate) fn on_event(
        &self,
        store: &mut Store<Sandbox>,
        kind: &str,
        payload: &str,
    ) -> Answer<()> {
        let Self::V2(guest) = self else {
            return Ok(Err(GuestError::unsupported("events")));
        };

        let event = types_v2::Event {
            kind: kind.to_owned(),
            payload: payload.to_owned(),
        };

        guest
            .call_on_event(store, &event)
            .map(|heard| heard.map_err(error_v2))
    }

    pub(crate) fn on_timer(&self, store: &mut Store<Sandbox>, name: &str) -> Answer<()> {
        let Self::V2(guest) = self else {
            return Ok(Err(GuestError::unsupported("timers")));
        };

        guest
            .call_on_timer(store, name)
            .map(|woke| woke.map_err(error_v2))
    }

    pub(crate) fn merge(
        &self,
        store: &mut Store<Sandbox>,
        kind: &str,
        sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Answer<Vec<u8>> {
        let Self::V2(guest) = self else {
            return Ok(Err(GuestError::unsupported("documents")));
        };

        let (base, ours, theirs) = sides;

        guest
            .call_merge(store, kind, base, ours, theirs)
            .map(|merged| merged.map_err(error_v2))
    }
}

fn described_v1(action: types_v1::Action) -> DescribedAction {
    let risk = match action.risk {
        types_v1::Risk::Read => Risk::Read,
        types_v1::Risk::Write => Risk::Write,
        types_v1::Risk::Privileged => Risk::Privileged,
    };

    DescribedAction {
        id: action.id,
        title: action.title,
        risk,
        takes_target: action.takes_target,
        params_schema: action.params_schema,
    }
}

fn described_v2(action: types_v2::Action) -> DescribedAction {
    let risk = match action.risk {
        types_v2::Risk::Read => Risk::Read,
        types_v2::Risk::Write => Risk::Write,
        types_v2::Risk::Privileged => Risk::Privileged,
    };

    DescribedAction {
        id: action.id,
        title: action.title,
        risk,
        takes_target: action.takes_target,
        params_schema: action.params_schema,
    }
}

fn error_v1(error: types_v1::Error) -> GuestError {
    GuestError::new(error.code, error.message)
}

fn error_v2(error: types_v2::Error) -> GuestError {
    GuestError::new(error.code, error.message)
}
