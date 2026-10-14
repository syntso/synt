use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, Trap, UpdateDeadline};

use super::budget::Budget;
use super::host::add_wasi_to_linker;
use super::ticker::Ticker;
use super::{Compiled, Exports, Imports, Sandbox};
use crate::ExtensionError;
use crate::bindings::{v1, v2};
use crate::manifest::Api;

const FUEL_PER_CALL: u64 = 10_000_000_000;
const TIME_PER_CALL: Duration = Duration::from_secs(10);
const INTERFACE_PREFIX: &str = "synt:extension/";

#[derive(Clone)]
pub struct Runtime {
    engine: Engine,
    linker: Arc<Linker<Sandbox>>,
    fuel: u64,
    time: Duration,
    _ticker: Arc<Ticker>,
}

impl Runtime {
    pub fn new() -> Result<Self, ExtensionError> {
        let engine = Engine::new(&config())?;
        let linker = linker(&engine)?;
        let ticker = Ticker::start(engine.clone())?;

        Ok(Self {
            engine,
            linker: Arc::new(linker),
            fuel: FUEL_PER_CALL,
            time: TIME_PER_CALL,
            _ticker: Arc::new(ticker),
        })
    }

    #[must_use]
    pub fn with_limits(mut self, fuel: u64, time: Duration) -> Self {
        self.fuel = fuel;
        self.time = time;
        self
    }

    pub(crate) fn compile(&self, api: Api, bytes: &[u8]) -> Result<Compiled, ExtensionError> {
        let component = Component::new(&self.engine, bytes)?;
        self.check_interface_version(&component, api)?;

        let instance = self.linker.instantiate_pre(&component)?;

        Compiled::prepare(api, instance).map_err(|error| {
            ExtensionError::Component(format!(
                "the component is not an extension for api {api}: {error:#}"
            ))
        })
    }

    pub(crate) fn enter<T>(
        &self,
        compiled: &Compiled,
        imports: Box<dyn Imports>,
        stop: CancellationToken,
        work: impl FnOnce(&mut Store<Sandbox>, &dyn Exports) -> wasmtime::Result<T>,
    ) -> Result<T, ExtensionError> {
        let budget = Budget::new(self.time, stop);
        let mut store = self.store(Sandbox::new(budget, imports))?;

        let entered = compiled
            .instance(&mut store)
            .and_then(|guest| work(&mut store, guest.as_ref()));

        entered.map_err(|error| store.data().budget().blame(&error))
    }

    fn store(&self, sandbox: Sandbox) -> Result<Store<Sandbox>, ExtensionError> {
        let mut store = Store::new(&self.engine, sandbox);

        store.limiter(|sandbox| sandbox.limits());
        store.set_fuel(self.fuel)?;
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(|context| {
            let budget = context.data().budget();

            if budget.stopped() || budget.spent() {
                return Err(Trap::Interrupt.into());
            }

            Ok(UpdateDeadline::Continue(1))
        });

        Ok(store)
    }

    fn check_interface_version(
        &self,
        component: &Component,
        api: Api,
    ) -> Result<(), ExtensionError> {
        let version = format!("@{api}.0");
        let component_type = component.component_type();

        let other = component_type
            .imports(&self.engine)
            .map(|(name, _)| name)
            .filter(|name| name.starts_with(INTERFACE_PREFIX))
            .find(|name| !name.ends_with(&version));

        match other {
            Some(name) => Err(ExtensionError::Component(format!(
                "the component is not an extension for api {api}: it imports {name}"
            ))),
            None => Ok(()),
        }
    }
}

fn config() -> Config {
    let mut config = Config::new();

    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);

    config
}

fn linker(engine: &Engine) -> wasmtime::Result<Linker<Sandbox>> {
    let mut linker = Linker::new(engine);

    add_wasi_to_linker(&mut linker)?;
    v1::Extension::add_to_linker::<Sandbox, HasSelf<Sandbox>>(&mut linker, |sandbox| sandbox)?;
    v2::Extension::add_to_linker::<Sandbox, HasSelf<Sandbox>>(&mut linker, |sandbox| sandbox)?;

    Ok(linker)
}
