use wasmtime::component::ResourceTable;
use wasmtime::{StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::runtime::Imports;
use crate::runtime::budget::Budget;
use crate::runtime::quota::Quota;

const MEMORY_LIMIT: usize = 64 * 1024 * 1024;
const MEMORIES: usize = 4;
const INSTANCES: usize = 32;
const TABLES: usize = 8;
const TABLE_ELEMENTS: usize = 100_000;

pub(crate) struct Sandbox {
    wasi: WasiCtx,
    table: ResourceTable,
    limits: StoreLimits,
    budget: Budget,
    quota: Quota,
    imports: Box<dyn Imports>,
}

impl Sandbox {
    pub(in crate::runtime) fn new(budget: Budget, imports: Box<dyn Imports>) -> Self {
        Self {
            wasi: empty_wasi(),
            table: ResourceTable::new(),
            limits: limits(),
            budget,
            quota: Quota::new(),
            imports,
        }
    }

    pub(in crate::runtime) fn limits(&mut self) -> &mut StoreLimits {
        &mut self.limits
    }

    pub(in crate::runtime) fn budget(&self) -> &Budget {
        &self.budget
    }

    pub(super) fn reach_out<T>(
        &mut self,
        work: impl FnOnce(&mut dyn Imports) -> T,
    ) -> wasmtime::Result<T> {
        self.quota.take_call()?;

        let imports = &mut self.imports;
        Ok(self.budget.outside(|| work(imports.as_mut())))
    }

    pub(super) fn keep<T>(
        &mut self,
        work: impl FnOnce(&mut dyn Imports) -> T,
    ) -> wasmtime::Result<T> {
        self.quota.take_storage()?;

        let imports = &mut self.imports;
        Ok(self.budget.outside(|| work(imports.as_mut())))
    }

    pub(super) fn tell<T>(
        &mut self,
        work: impl FnOnce(&mut dyn Imports) -> T,
    ) -> wasmtime::Result<T> {
        self.quota.take_message()?;

        Ok(work(self.imports.as_mut()))
    }

    pub(super) fn quietly<T>(&mut self, work: impl FnOnce(&mut dyn Imports) -> T) -> T {
        work(self.imports.as_mut())
    }
}

impl WasiView for Sandbox {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn limits() -> StoreLimits {
    StoreLimitsBuilder::new()
        .memory_size(MEMORY_LIMIT)
        .memories(MEMORIES)
        .instances(INSTANCES)
        .tables(TABLES)
        .table_elements(TABLE_ELEMENTS)
        .trap_on_grow_failure(true)
        .build()
}

fn empty_wasi() -> WasiCtx {
    WasiCtxBuilder::new()
        .allow_tcp(false)
        .allow_udp(false)
        .allow_ip_name_lookup(false)
        .build()
}
