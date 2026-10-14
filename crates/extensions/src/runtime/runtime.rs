use std::sync::Arc;
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, Trap, UpdateDeadline};

use super::budget::Budget;
use super::ticker::Ticker;
use super::{Compiled, GuestCall, Imports, QuotaSpent, Sandbox, wasi};
use crate::ExtensionError;
use crate::bindings::synt::extension::types::{Action, Error};
use crate::bindings::{Extension, ExtensionPre};

const FUEL_PER_CALL: u64 = 10_000_000_000;
const TIME_PER_CALL: Duration = Duration::from_secs(10);

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
        let engine = Engine::new(&config()).map_err(|error| component_error(&error))?;
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

    pub(crate) fn compile(&self, bytes: &[u8]) -> Result<Compiled, ExtensionError> {
        let component =
            Component::new(&self.engine, bytes).map_err(|error| component_error(&error))?;
        let instance = self
            .linker
            .instantiate_pre(&component)
            .map_err(|error| component_error(&error))?;
        let pre = ExtensionPre::new(instance).map_err(|error| component_error(&error))?;

        Ok(Compiled::new(pre))
    }

    pub(crate) fn describe(
        &self,
        compiled: &Compiled,
        imports: Box<dyn Imports>,
    ) -> Result<Vec<Action>, ExtensionError> {
        let stop = CancellationToken::new();

        self.enter(compiled, imports, stop, |store, guest| {
            guest.call_describe(store)
        })
    }

    pub(crate) fn run(
        &self,
        compiled: &Compiled,
        imports: Box<dyn Imports>,
        call: &GuestCall,
        stop: CancellationToken,
    ) -> Result<Result<String, Error>, ExtensionError> {
        self.enter(compiled, imports, stop, |store, guest| {
            guest.call_run(store, &call.action, call.target.as_deref(), &call.params)
        })
    }

    fn enter<T>(
        &self,
        compiled: &Compiled,
        imports: Box<dyn Imports>,
        stop: CancellationToken,
        work: impl FnOnce(&mut Store<Sandbox>, &Extension) -> wasmtime::Result<T>,
    ) -> Result<T, ExtensionError> {
        let budget = Budget::new(self.time, stop);
        let mut store = self.store(Sandbox::new(budget, imports))?;

        let entered = compiled
            .pre()
            .instantiate(&mut store)
            .and_then(|guest| work(&mut store, &guest));

        entered.map_err(|error| failure(&error, store.data().budget()))
    }

    fn store(&self, sandbox: Sandbox) -> Result<Store<Sandbox>, ExtensionError> {
        let mut store = Store::new(&self.engine, sandbox);

        store.limiter(|sandbox| sandbox.limits());
        store
            .set_fuel(self.fuel)
            .map_err(|error| component_error(&error))?;
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
}

fn config() -> Config {
    let mut config = Config::new();

    config.wasm_component_model(true);
    config.consume_fuel(true);
    config.epoch_interruption(true);

    config
}

fn linker(engine: &Engine) -> Result<Linker<Sandbox>, ExtensionError> {
    let mut linker = Linker::new(engine);

    wasi::add_to_linker(&mut linker).map_err(|error| component_error(&error))?;
    Extension::add_to_linker::<Sandbox, HasSelf<Sandbox>>(&mut linker, |sandbox| sandbox)
        .map_err(|error| component_error(&error))?;

    Ok(linker)
}

fn failure(error: &wasmtime::Error, budget: &Budget) -> ExtensionError {
    if let Some(spent) = error.downcast_ref::<QuotaSpent>() {
        return ExtensionError::OverQuota(spent.to_string());
    }

    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => ExtensionError::OutOfFuel,
        Some(Trap::Interrupt) if budget.stopped() => ExtensionError::Stopped,
        Some(Trap::Interrupt) => ExtensionError::TimedOut(budget.limit()),
        _ => ExtensionError::Trapped(format!("{error:#}")),
    }
}

fn component_error(error: &wasmtime::Error) -> ExtensionError {
    ExtensionError::Component(format!("{error:#}"))
}
