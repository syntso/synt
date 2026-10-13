use std::env;
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;

use synt_actions::Registry;
use synt_core::{Core, Ping, ReadOnlyGate, Server, VERSION};
use synt_journal::{Journal, JournalWriter, MemoryKeyStore};
use synt_platform::{Endpoint, shutdown_signal};
use tempfile::TempDir;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("synt-core: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let endpoint = endpoint()?;
    let scratch = scratch_directory()?;

    let keys = Arc::new(MemoryKeyStore::new()?);
    let journal = Journal::create(scratch.path().join("journal.sqlite"), keys)?;
    let journal = JournalWriter::spawn(journal)?;

    let mut registry = Registry::new();
    registry.register(Ping)?;

    let core = Core::new(registry, journal, ReadOnlyGate);
    let server = Server::bind(&endpoint, core)?;

    eprintln!(
        "synt-core {VERSION} listening on {}",
        endpoint.address().to_string_lossy()
    );
    eprintln!(
        "journal is temporary until platform key storage lands: {}",
        scratch.path().display()
    );

    server.run(shutdown_signal()).await;

    scratch.close()?;
    Ok(())
}

fn endpoint() -> Result<Endpoint, Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);

    match (arguments.next(), arguments.next()) {
        (None, _) => Ok(Endpoint::for_user()?),
        (Some(flag), Some(address)) if flag == "--endpoint" => Ok(Endpoint::new(address)),
        (Some(other), _) => Err(format!("unknown argument {}", other.to_string_lossy()).into()),
    }
}

fn scratch_directory() -> Result<TempDir, Box<dyn Error>> {
    let directory = tempfile::Builder::new().prefix("synt-core-").tempdir()?;
    Ok(directory)
}
