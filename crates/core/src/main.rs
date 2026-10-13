use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::{self, Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use synt_actions::Registry;
use synt_core::Files;
use synt_core::{AlwaysGrants, GrantList, GrantRevoke, GrantSeal, Projects, State};
use synt_core::{
    Broker, Core, JournalPage, JournalVerify, Ping, Server, SshConnections, SshExec, SshWarm,
    Storage, VERSION,
};
use synt_core::{IdleWatch, LoginCommand};
use synt_core::{
    ProjectClose, ProjectList, ProjectOpen, ResourceGet, ResourceList, ResourcePut, ResourceRemove,
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
use tokio::task;

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
const STAY_FLAG: &str = "--stay";
const IDLE_FLAG: &str = "--idle-minutes";
const IDLE_LIMIT: Duration = Duration::from_mins(10);

struct Arguments {
    endpoint: Option<Endpoint>,
    data: Option<PathBuf>,
    weak_vault: WeakVault,
    idle: Option<Duration>,
}

async fn run() -> Result<(), Box<dyn Error>> {
    if let Some(command) = login_command()? {
        return login(command).await;
    }

    let arguments = arguments()?;

    let endpoint = match arguments.endpoint {
        Some(endpoint) => endpoint,
        None => Endpoint::for_user()?,
    };

    let data = match arguments.data {
        Some(data) => path::absolute(data)?,
        None => data_directory()?,
    };

    let (journal, state, seal) = journal(&data, arguments.weak_vault)?;
    let journal = JournalWriter::spawn(journal)?;

    let projects = Projects::start(state.clone())?;
    let always = AlwaysGrants::load(state, seal)?;

    let pool = Pool::new();

    let mut registry = Registry::new();
    registry.register(Ping)?;
    registry.register(SshExec::new(pool.clone(), projects.clone()))?;

    let tunnels = Tunnels::new(pool.clone());
    registry.register(TunnelOpen::new(tunnels.clone(), projects.clone()))?;
    registry.register(TunnelList::new(tunnels.clone()))?;
    registry.register(TunnelClose::new(tunnels.clone()))?;

    Files::new(pool.clone(), projects.clone()).register(&mut registry)?;

    let terminals = Terminals::new(journal.clone());
    registry.register(SshSessionOpen::new(
        pool.clone(),
        terminals.clone(),
        projects.clone(),
    ))?;
    registry.register(SessionAttach::new(terminals.clone()))?;
    registry.register(SessionResize::new(terminals.clone()))?;
    registry.register(SessionClose::new(terminals.clone()))?;
    registry.register(SessionList::new(terminals.clone()))?;

    registry.register(SshWarm::new(pool.clone(), projects.clone()))?;
    registry.register(SshConnections::new(pool))?;
    registry.register(JournalPage::new(journal.clone()))?;
    registry.register(JournalVerify::new(journal.clone()))?;

    registry.register(ProjectOpen::new(projects.clone()))?;
    registry.register(ProjectClose::new(projects.clone()))?;
    registry.register(ProjectList::new(projects.clone()))?;
    registry.register(ResourceList::new(projects.clone()))?;
    registry.register(ResourceGet::new(projects.clone()))?;
    registry.register(ResourcePut::new(projects.clone()))?;
    registry.register(ResourceRemove::new(projects))?;
    registry.register(GrantList::new(always.clone()))?;
    registry.register(GrantRevoke::new(always.clone(), journal.clone()))?;

    let core = Core::new(registry, journal, Broker::with_always(always));
    let server = Server::bind(&endpoint, core, apps()?)?;

    eprintln!(
        "synt-core {VERSION} listening on {}",
        endpoint.address().to_string_lossy()
    );

    let idle = IdleWatch::new(arguments.idle)
        .watch(server.activity())
        .watch(terminals.clone())
        .watch(tunnels);

    let stopping = async {
        tokio::select! {
            () = shutdown_signal() => {}
            () = idle.idle() => eprintln!("synt-core: idle, stopping"),
        }
        terminals.close_all().await;
    };
    server.run(stopping).await;

    Ok(())
}

fn journal(
    data: &Path,
    weak_vault: WeakVault,
) -> Result<(Journal, State, GrantSeal), Box<dyn Error>> {
    let vault = open_vault(data, weak_vault)?;
    let place = vault.place();
    eprintln!("journal keys are kept in {place}");

    if let Some(weakness) = place.weakness() {
        eprintln!("warning: {weakness}");
        eprintln!("warning: {}", weak_vault_reason(weak_vault));
    }

    let storage = Storage::prepare(data, vault)?;
    let journal = storage.journal()?;
    let state = storage.state()?;
    let seal = storage.grant_seal()?;

    eprintln!(
        "journal {} at {}, last event {}",
        journal.id(),
        journal.path().display(),
        journal.anchors().head.seq
    );

    Ok((journal, state, seal))
}

fn apps() -> Result<AppCheck, Box<dyn Error>> {
    let apps = AppCheck::for_this_core()?;
    eprintln!("a client that says it is the app acts for the person only if {apps}");

    if let Some(weakness) = apps.weakness() {
        eprintln!("warning: {weakness}");
    }

    Ok(apps)
}

fn login_command() -> Result<Option<LoginCommand>, Box<dyn Error>> {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();

    let Some((first, rest)) = arguments.split_first() else {
        return Ok(None);
    };
    if first != LoginCommand::NAME {
        return Ok(None);
    }

    Ok(Some(LoginCommand::parse(rest)?))
}

async fn login(command: LoginCommand) -> Result<(), Box<dyn Error>> {
    let report = task::spawn_blocking(move || command.run()).await??;
    println!("{report}");

    Ok(())
}

fn arguments() -> Result<Arguments, Box<dyn Error>> {
    let mut parsed = Arguments {
        endpoint: None,
        data: None,
        weak_vault: WeakVault::Refuse,
        idle: Some(IDLE_LIMIT),
    };
    let mut arguments = env::args_os().skip(1);

    while let Some(argument) = arguments.next() {
        if argument == WEAK_VAULT_FLAG {
            parsed.weak_vault = WeakVault::Allow;
            continue;
        }

        if argument == STAY_FLAG {
            parsed.idle = None;
            continue;
        }

        if argument != ENDPOINT_FLAG && argument != DATA_FLAG && argument != IDLE_FLAG {
            return Err(format!("unknown argument {}", argument.to_string_lossy()).into());
        }

        let value = arguments
            .next()
            .ok_or_else(|| format!("{} needs a value", argument.to_string_lossy()))?;

        if argument == ENDPOINT_FLAG {
            parsed.endpoint = Some(Endpoint::new(value));
        } else if argument == DATA_FLAG {
            parsed.data = Some(PathBuf::from(value));
        } else if parsed.idle.is_some() {
            parsed.idle = Some(idle_limit(&value)?);
        }
    }

    Ok(parsed)
}

fn idle_limit(value: &OsStr) -> Result<Duration, Box<dyn Error>> {
    let minutes: u64 = value
        .to_str()
        .and_then(|text| text.parse().ok())
        .ok_or_else(|| format!("{IDLE_FLAG} needs a whole number of minutes"))?;

    Ok(Duration::from_mins(minutes))
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
