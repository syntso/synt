use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use synt_actions::Context;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use wasmtime::Store;

use super::{ActionSpec, Outside, Services, Visit};
use crate::ExtensionError;
use crate::background::{Timers, Wake};
use crate::interface::GuestError;
use crate::manifest::Manifest;
use crate::runtime::{Compiled, GuestCall, Instance, Runtime, Sandbox};

const MOST_ACTIONS: usize = 64;

pub(crate) type Answer<T> = Result<Result<T, GuestError>, ExtensionError>;

pub(crate) struct Guest {
    manifest: Manifest,
    compiled: Compiled,
    runtime: Runtime,
    services: Arc<Services>,
    timers: Timers,
    actions: OnceLock<BTreeSet<String>>,
}

impl Guest {
    pub(crate) fn new(
        manifest: Manifest,
        compiled: Compiled,
        runtime: Runtime,
        services: Arc<Services>,
    ) -> Self {
        Self {
            manifest,
            compiled,
            runtime,
            services,
            timers: Timers::default(),
            actions: OnceLock::new(),
        }
    }

    pub(crate) fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub(crate) fn services(&self) -> &Services {
        &self.services
    }

    pub(crate) fn timers(&self) -> &Timers {
        &self.timers
    }

    pub(crate) fn owns_action(&self, action: &str) -> bool {
        self.actions
            .get()
            .is_some_and(|actions| actions.contains(action))
    }

    pub(crate) fn describe(self: &Arc<Self>) -> Result<Vec<ActionSpec>, ExtensionError> {
        let described = self.enter(None, CancellationToken::new(), |store, instance| {
            instance.describe(store)
        })?;

        if described.is_empty() || described.len() > MOST_ACTIONS {
            return Err(ExtensionError::Component(format!(
                "an extension adds 1 to {MOST_ACTIONS} actions, this one describes {}",
                described.len()
            )));
        }

        let specs = described
            .iter()
            .map(|action| ActionSpec::from_described(&self.manifest.id, action))
            .collect::<Result<Vec<_>, _>>()?;

        check_unique(&specs)?;
        let _ = self.actions.set(
            specs
                .iter()
                .map(|spec| spec.id.as_str().to_owned())
                .collect(),
        );

        Ok(specs)
    }

    pub(crate) fn run(
        self: &Arc<Self>,
        call: &GuestCall,
        context: Context,
        runtime: Handle,
    ) -> Answer<String> {
        let stop = context.stop().clone();
        let outside = self.outside(Some(context), runtime)?;

        self.enter(Some(outside), stop, |store, instance| {
            instance.run(store, call)
        })
    }

    pub(crate) fn render(
        self: &Arc<Self>,
        view: &str,
        call: &GuestCall,
        context: Context,
        runtime: Handle,
    ) -> Answer<String> {
        let stop = context.stop().clone();
        let outside = self.outside(Some(context), runtime)?;
        let target = call.target.as_deref();

        self.enter(Some(outside), stop, |store, instance| {
            instance.render(store, view, target, &call.params)
        })
    }

    pub(crate) fn panel_message(
        self: &Arc<Self>,
        panel: &str,
        message: &str,
        context: Context,
        runtime: Handle,
    ) -> Answer<String> {
        let stop = context.stop().clone();
        let outside = self.outside(Some(context), runtime)?;

        self.enter(Some(outside), stop, |store, instance| {
            instance.panel_message(store, panel, message)
        })
    }

    pub(crate) fn asset(self: &Arc<Self>, path: &str) -> Result<Option<Vec<u8>>, ExtensionError> {
        self.enter(None, CancellationToken::new(), |store, instance| {
            instance.asset(store, path)
        })
    }

    pub(crate) fn merge(
        self: &Arc<Self>,
        kind: &str,
        sides: (Option<&[u8]>, &[u8], &[u8]),
    ) -> Answer<Vec<u8>> {
        self.enter(None, CancellationToken::new(), |store, instance| {
            instance.merge(store, kind, sides)
        })
    }

    pub(crate) fn wake(
        self: &Arc<Self>,
        wake: &Wake,
        stop: CancellationToken,
        runtime: Handle,
    ) -> Answer<()> {
        let outside = self.outside(None, runtime)?;

        self.enter(Some(outside), stop, |store, instance| match wake {
            Wake::Start => instance.start(store),
            Wake::Timer(name) => instance.on_timer(store, name),
            Wake::Event(event) => {
                let payload = event.payload.to_string();
                instance.on_event(store, event.topic.as_str(), &payload)
            }
        })
    }

    fn outside(&self, origin: Option<Context>, runtime: Handle) -> Result<Outside, ExtensionError> {
        let caller = self.services.caller.get().cloned().ok_or_else(|| {
            ExtensionError::Refused("the core is not ready to run extensions yet".to_owned())
        })?;

        Ok(Outside {
            caller,
            origin,
            runtime,
        })
    }

    fn enter<T>(
        self: &Arc<Self>,
        outside: Option<Outside>,
        stop: CancellationToken,
        work: impl FnOnce(&mut Store<Sandbox>, &Instance) -> wasmtime::Result<T>,
    ) -> Result<T, ExtensionError> {
        let visit = Visit::new(Arc::clone(self), outside);

        self.runtime
            .enter(&self.compiled, Box::new(visit), stop, work)
    }
}

fn check_unique(specs: &[ActionSpec]) -> Result<(), ExtensionError> {
    let mut seen = BTreeSet::new();

    for spec in specs {
        if !seen.insert(spec.id.as_str()) {
            return Err(ExtensionError::Component(format!(
                "action {} is described twice",
                spec.id
            )));
        }
    }

    Ok(())
}
