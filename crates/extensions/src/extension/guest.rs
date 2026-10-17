use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use synt_actions::Context;
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use wasmtime::Store;

use super::{ActionSpec, Outside, Services, Visit};
use crate::ExtensionError;
use crate::background::{Timers, Wake};
use crate::documents::MergeSides;
use crate::interface::{GuestError, Reply};
use crate::manifest::Manifest;
use crate::runtime::{Compiled, Exports, GuestCall, Runtime, Sandbox};

const MOST_ACTIONS: usize = 64;

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

    pub(crate) fn id(&self) -> &str {
        self.manifest.id().as_str()
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

    pub(crate) fn require(&self, granted: bool, missing: &str) -> Result<(), GuestError> {
        if granted {
            return Ok(());
        }

        Err(GuestError::missing_capability(self.id(), missing))
    }

    pub(crate) fn check_contributed_actions(&self) -> Result<(), ExtensionError> {
        let missing = self
            .manifest
            .contributes()
            .actions()
            .find(|action| !self.owns_action(action));

        match missing {
            Some(action) => Err(ExtensionError::Undescribed(action.to_owned())),
            None => Ok(()),
        }
    }

    pub(crate) fn describe(self: &Arc<Self>) -> Result<Vec<ActionSpec>, ExtensionError> {
        let described = self.enter_alone(|store, exports| exports.describe(store))?;

        if described.is_empty() || described.len() > MOST_ACTIONS {
            return Err(ExtensionError::ActionCount {
                most: MOST_ACTIONS,
                found: described.len(),
            });
        }

        let specs = described
            .iter()
            .map(|action| ActionSpec::from_described(self.id(), action))
            .collect::<Result<Vec<_>, _>>()?;

        self.remember_actions(&specs)?;

        Ok(specs)
    }

    pub(crate) fn run(
        self: &Arc<Self>,
        call: &GuestCall,
        context: Context,
        runtime: Handle,
    ) -> Result<Reply, ExtensionError> {
        let text = self.enter_for(context, runtime, |store, exports| exports.run(store, call))?;

        Ok(Reply::new(text))
    }

    pub(crate) fn render(
        self: &Arc<Self>,
        view: &str,
        call: &GuestCall,
        context: Context,
        runtime: Handle,
    ) -> Result<String, ExtensionError> {
        self.enter_for(context, runtime, |store, exports| {
            exports.render(store, view, call.target(), call.params())
        })
    }

    pub(crate) fn panel_message(
        self: &Arc<Self>,
        panel: &str,
        message: &str,
        context: Context,
        runtime: Handle,
    ) -> Result<Reply, ExtensionError> {
        let text = self.enter_for(context, runtime, |store, exports| {
            exports.panel_message(store, panel, message)
        })?;

        Ok(Reply::new(text))
    }

    pub(crate) fn asset(self: &Arc<Self>, path: &str) -> Result<Option<Vec<u8>>, ExtensionError> {
        self.enter_alone(|store, exports| exports.asset(store, path))
    }

    pub(crate) fn merge(
        self: &Arc<Self>,
        kind: &str,
        sides: MergeSides<'_>,
    ) -> Result<Vec<u8>, ExtensionError> {
        self.enter_alone(|store, exports| exports.merge(store, kind, sides))
    }

    pub(crate) fn wake(
        self: &Arc<Self>,
        wake: &Wake,
        stop: CancellationToken,
        runtime: Handle,
    ) -> Result<(), ExtensionError> {
        let outside = self.outside(None, runtime)?;

        self.enter(Some(outside), stop, |store, exports| match wake {
            Wake::Start => exports.start(store),
            Wake::Timer(name) => exports.on_timer(store, &name.to_string()),
            Wake::Event(event) => {
                let payload = event.payload().to_string();
                exports.on_event(store, event.topic().as_str(), &payload)
            }
        })
    }

    fn remember_actions(&self, specs: &[ActionSpec]) -> Result<(), ExtensionError> {
        let mut ids = BTreeSet::new();

        for spec in specs {
            if !ids.insert(spec.id().as_str().to_owned()) {
                return Err(ExtensionError::DuplicateAction(spec.id().to_string()));
            }
        }

        self.actions
            .set(ids)
            .map_err(|_| ExtensionError::DescribedTwice(self.id().to_owned()))
    }

    fn outside(&self, origin: Option<Context>, runtime: Handle) -> Result<Outside, ExtensionError> {
        let link = self.services.link().ok_or(ExtensionError::NotConnected)?;

        Ok(Outside::new(Arc::clone(link.caller()), origin, runtime))
    }

    fn enter_for<T>(
        self: &Arc<Self>,
        context: Context,
        runtime: Handle,
        work: impl FnOnce(&mut Store<Sandbox>, &dyn Exports) -> wasmtime::Result<T>,
    ) -> Result<T, ExtensionError> {
        let stop = context.stop().clone();
        let outside = self.outside(Some(context), runtime)?;

        self.enter(Some(outside), stop, work)
    }

    fn enter_alone<T>(
        self: &Arc<Self>,
        work: impl FnOnce(&mut Store<Sandbox>, &dyn Exports) -> wasmtime::Result<T>,
    ) -> Result<T, ExtensionError> {
        self.enter(None, CancellationToken::new(), work)
    }

    fn enter<T>(
        self: &Arc<Self>,
        outside: Option<Outside>,
        stop: CancellationToken,
        work: impl FnOnce(&mut Store<Sandbox>, &dyn Exports) -> wasmtime::Result<T>,
    ) -> Result<T, ExtensionError> {
        let visit = Visit::new(self, outside);

        self.runtime
            .enter(&self.compiled, Box::new(visit), stop, work)
    }
}
