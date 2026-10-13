use std::env;
use std::error::Error;
use std::process::ExitCode;
use std::sync::Arc;

use synt_actions::Registry;
use synt_core::{Broker, Core, JournalPage, JournalVerify, Ping, Server, SshExec, VERSION};
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

const ENDPOINT_FLAG: &str = "--endpoint";
const TRUST_FLAG: &str = "--trust-local-app";

async fn run() -> Result<(), Box<dyn Error>> {
    let (endpoint, trust_local_app) = arguments()?;
    let scratch = scratch_directory()?;

    let keys = Arc::new(MemoryKeyStore::new()?);
    let journal = Journal::create(scratch.path().join("journal.sqlite"), keys)?;
    let journal = JournalWriter::spawn(journal)?;

    let mut registry = Registry::new();
    registry.register(Ping)?;
    registry.register(SshExec::new())?;
    registry.register(JournalPage::new(journal.clone()))?;
    registry.register(JournalVerify::new(journal.clone()))?;

    let core = Core::new(registry, journal, Broker::new());
    let server = trusting(Server::bind(&endpoint, core)?, trust_local_app);

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

fn arguments() -> Result<(Endpoint, bool), Box<dyn Error>> {
    let mut endpoint = None;
    let mut trust_local_app = false;
    let mut arguments = env::args_os().skip(1);

    while let Some(argument) = arguments.next() {
        if argument == TRUST_FLAG {
            trust_local_app = true;
            continue;
        }

        if argument != ENDPOINT_FLAG {
            return Err(format!("unknown argument {}", argument.to_string_lossy()).into());
        }

        let address = arguments
            .next()
            .ok_or_else(|| format!("{ENDPOINT_FLAG} needs an address"))?;
        endpoint = Some(Endpoint::new(address));
    }

    let endpoint = match endpoint {
        Some(endpoint) => endpoint,
        None => Endpoint::for_user()?,
    };

    Ok((endpoint, trust_local_app))
}

fn trusting(server: Server, trust_local_app: bool) -> Server {
    if !trust_local_app {
        eprintln!(
            "apps are not verified yet, so a local app acts as the agent \"app\" and cannot \
             approve; pass {TRUST_FLAG} to let it act for the person during development"
        );
        return server;
    }

    eprintln!("{TRUST_FLAG}: any local process that says it is the app acts for the person");
    server.trusting_local_apps()
}

fn scratch_directory() -> Result<TempDir, Box<dyn Error>> {
    let directory = tempfile::Builder::new().prefix("synt-core-").tempdir()?;
    Ok(directory)
}
