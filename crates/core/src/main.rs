use std::env;
use std::error::Error;
use std::path::{self, Path, PathBuf};
use std::process::ExitCode;

use synt_actions::Registry;
use synt_core::Files;
use synt_core::{
    Broker, Core, JournalPage, JournalVerify, Ping, Server, SshConnections, SshExec, Storage,
    VERSION,
};
use synt_core::{
    SessionAttach, SessionClose, SessionList, SessionResize, SshSessionOpen, Terminals,
};
use synt_core::{TunnelClose, TunnelList, TunnelOpen};
use synt_journal::{Journal, JournalWriter};
use synt_platform::{
    AppCheck, Endpoint, WEAK_VAULT_FLAG, WeakVault, data_directory, open_vault, shutdown_signal,
};
use synt_ssh::Pool;
use synt_ssh::Tunnels;

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
const DATA_FLAG: &str = "--data";

struct Arguments {
    endpoint: Option<Endpoint>,
    data: Option<PathBuf>,
    weak_vault: WeakVault,
}

async fn run() -> Result<(), Box<dyn Error>> {
    let arguments = arguments()?;

    let endpoint = match arguments.endpoint {
        Some(endpoint) => endpoint,
        None => Endpoint::for_user()?,
    };

    let data = match arguments.data {
        Some(data) => path::absolute(data)?,
        None => data_directory()?,
    };

    let journal = JournalWriter::spawn(journal(&data, arguments.weak_vault)?)?;

    let pool = Pool::new();

    let mut registry = Registry::new();
    registry.register(Ping)?;
    registry.register(SshExec::new(pool.clone()))?;

    let tunnels = Tunnels::new(pool.clone());
    registry.register(TunnelOpen::new(tunnels.clone()))?;
    registry.register(TunnelList::new(tunnels.clone()))?;
    registry.register(TunnelClose::new(tunnels))?;

    Files::new(pool.clone()).register(&mut registry)?;

    let terminals = Terminals::new(journal.clone());
    registry.register(SshSessionOpen::new(pool.clone(), terminals.clone()))?;
    registry.register(SessionAttach::new(terminals.clone()))?;
    registry.register(SessionResize::new(terminals.clone()))?;
    registry.register(SessionClose::new(terminals.clone()))?;
    registry.register(SessionList::new(terminals.clone()))?;

    registry.register(SshConnections::new(pool))?;
    registry.register(JournalPage::new(journal.clone()))?;
    registry.register(JournalVerify::new(journal.clone()))?;

    let core = Core::new(registry, journal, Broker::new());
    let server = Server::bind(&endpoint, core, apps()?)?;

    eprintln!(
        "synt-core {VERSION} listening on {}",
        endpoint.address().to_string_lossy()
    );

    let stopping = async {
        shutdown_signal().await;
        terminals.close_all().await;
    };
    server.run(stopping).await;

    Ok(())
}

fn journal(data: &Path, weak_vault: WeakVault) -> Result<Journal, Box<dyn Error>> {
    let vault = open_vault(data, weak_vault)?;
    let place = vault.place();
    eprintln!("journal keys are kept in {place}");

    if let Some(weakness) = place.weakness() {
        eprintln!("warning: {weakness}");
        eprintln!("warning: {}", weak_vault_reason(weak_vault));
    }

    let journal = Storage::prepare(data, vault)?.journal()?;

    eprintln!(
        "journal {} at {}, last event {}",
        journal.id(),
        journal.path().display(),
        journal.anchors().head.seq
    );

    Ok(journal)
}

fn apps() -> Result<AppCheck, Box<dyn Error>> {
    let apps = AppCheck::for_this_core()?;
    eprintln!("a client that says it is the app acts for the person only if {apps}");

    if let Some(weakness) = apps.weakness() {
        eprintln!("warning: {weakness}");
    }

    Ok(apps)
}

fn arguments() -> Result<Arguments, Box<dyn Error>> {
    let mut parsed = Arguments {
        endpoint: None,
        data: None,
        weak_vault: WeakVault::Refuse,
    };
    let mut arguments = env::args_os().skip(1);

    while let Some(argument) = arguments.next() {
        if argument == WEAK_VAULT_FLAG {
            parsed.weak_vault = WeakVault::Allow;
            continue;
        }

        if argument != ENDPOINT_FLAG && argument != DATA_FLAG {
            return Err(format!("unknown argument {}", argument.to_string_lossy()).into());
        }

        let value = arguments
            .next()
            .ok_or_else(|| format!("{} needs a value", argument.to_string_lossy()))?;

        if argument == ENDPOINT_FLAG {
            parsed.endpoint = Some(Endpoint::new(value));
        } else {
            parsed.data = Some(PathBuf::from(value));
        }
    }

    Ok(parsed)
}

fn weak_vault_reason(weak_vault: WeakVault) -> String {
    match weak_vault {
        WeakVault::Allow => format!("allowed by {WEAK_VAULT_FLAG}, which is for development only"),
        WeakVault::Refuse => format!(
            "allowed only because this is a debug build; a release build refuses this \
             without {WEAK_VAULT_FLAG}"
        ),
    }
}
