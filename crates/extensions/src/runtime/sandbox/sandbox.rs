use wasmtime::component::ResourceTable;
use wasmtime::{StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use super::Imports;
use super::budget::Budget;
use super::quota::Quota;
use crate::bindings::ExtensionImports;
use crate::bindings::synt::extension::types::{self, Error, Level};

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
    pub(super) fn new(budget: Budget, imports: Box<dyn Imports>) -> Self {
        Self {
            wasi: empty_wasi(),
            table: ResourceTable::new(),
            limits: limits(),
            budget,
            quota: Quota::new(),
            imports,
        }
    }

    pub(super) fn limits(&mut self) -> &mut StoreLimits {
        &mut self.limits
    }

    pub(super) fn budget(&self) -> &Budget {
        &self.budget
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

impl types::Host for Sandbox {}

impl ExtensionImports for Sandbox {
    fn call(
        &mut self,
        action: String,
        target: Option<String>,
        params: String,
    ) -> wasmtime::Result<Result<String, Error>> {
        self.quota.take_call()?;

        let imports = &mut self.imports;
        let answer = self.budget.outside(|| imports.call(action, target, params));

        Ok(answer)
    }

    fn storage_get(&mut self, key: String) -> wasmtime::Result<Option<Vec<u8>>> {
        self.quota.take_storage()?;

        Ok(self.imports.storage_get(key))
    }

    fn storage_set(&mut self, key: String, value: Vec<u8>) -> wasmtime::Result<Result<(), Error>> {
        self.quota.take_storage()?;

        Ok(self.imports.storage_set(key, value))
    }

    fn log(&mut self, level: Level, message: String) -> wasmtime::Result<()> {
        self.imports.log(level, message);

        Ok(())
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
