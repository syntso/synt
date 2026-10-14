use wasmtime::component::{HasData, Linker, ResourceTable};
use wasmtime_wasi::WasiView;
use wasmtime_wasi::cli::{WasiCli, WasiCliView};
use wasmtime_wasi::clocks::{WasiClocks, WasiClocksView};
use wasmtime_wasi::p2::bindings::sync::io;
use wasmtime_wasi::p2::bindings::{cli, clocks, random};
use wasmtime_wasi::random::{WasiRandom, WasiRandomView};

use super::Sandbox;

struct Streams;

impl HasData for Streams {
    type Data<'a> = &'a mut ResourceTable;
}

pub(super) fn add_to_linker(linker: &mut Linker<Sandbox>) -> wasmtime::Result<()> {
    add_io(linker)?;
    add_cli(linker)?;
    add_clock_and_random(linker)
}

fn add_io(linker: &mut Linker<Sandbox>) -> wasmtime::Result<()> {
    io::error::add_to_linker::<Sandbox, Streams>(linker, table)?;
    io::poll::add_to_linker::<Sandbox, Streams>(linker, table)?;
    io::streams::add_to_linker::<Sandbox, Streams>(linker, table)
}

fn add_cli(linker: &mut Linker<Sandbox>) -> wasmtime::Result<()> {
    cli::environment::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::exit::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::stdin::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::stdout::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::stderr::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::terminal_input::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::terminal_output::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::terminal_stdin::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::terminal_stdout::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)?;
    cli::terminal_stderr::add_to_linker::<Sandbox, WasiCli>(linker, Sandbox::cli)
}

fn add_clock_and_random(linker: &mut Linker<Sandbox>) -> wasmtime::Result<()> {
    clocks::monotonic_clock::add_to_linker::<Sandbox, WasiClocks>(linker, Sandbox::clocks)?;
    random::random::add_to_linker::<Sandbox, WasiRandom>(linker, Sandbox::random)?;
    random::insecure::add_to_linker::<Sandbox, WasiRandom>(linker, Sandbox::random)?;
    random::insecure_seed::add_to_linker::<Sandbox, WasiRandom>(linker, Sandbox::random)
}

fn table(sandbox: &mut Sandbox) -> &mut ResourceTable {
    sandbox.ctx().table
}
