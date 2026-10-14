use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::path::{self, Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use synt_actions::{ActionError, Registry};
use synt_core::{
    AlwaysGrants, Broker, Core, CoreCaller, CoreScreen, EventFeed, Files, GrantList, GrantRevoke,
    IdleWatch, JournalPage, JournalVerify, KeyGenerate, KeyImport, KeyList, KeyRemove, Kube,
    LocalSessionOpen, LoginCommand, Ping, ProjectClose, ProjectList, ProjectOpen, Projects,
    ResourceGet, ResourceList, ResourcePut, ResourceRemove, Runbooks, SecretList, SecretPut,
    SecretRemove, Secrets, Server, SessionAttach, SessionBlocks, SessionClose, SessionIntegration,
    SessionList, SessionResize, SshConnections, SshExec, SshKeys, SshSessionOpen, SshWarm, Storage,
    Terminals, TunnelClose, TunnelList, TunnelOpen, VERSION,
};
use synt_extensions::{Extensions, Remote, Runtime};
use synt_journal::{Journal, JournalWriter};
use synt_platform::{
    AppCheck, Endpoint, WEAK_VAULT_FLAG, WeakVault, data_directory, open_vault, shutdown_signal,
};
use synt_ssh::{Pool, Tunnels};
use synt_sync::{SyncLoop, Syncer};
use tokio::task;

const ENDPOINT_FLAG: &str = "--endpoint";
const DATA_FLAG: &str = "--data";
const STAY_FLAG: &str = "--stay";
const IDLE_FLAG: &str = "--idle-minutes";
const IDLE_LIMIT: Duration = Duration::from_mins(10);
const SYNC_EVERY: Duration = Duration::from_secs(10);

struct Arguments {
    endpoint: Option<Endpoint>,
    data: Option<PathBuf>,
    weak_vault: WeakVault,
    idle: Option<Duration>,
}

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

    let (journal, storage) = journal(&data, arguments.weak_vault)?;
    let journal = JournalWriter::spawn(journal)?;
    let state = storage.state()?;
    let keys = SshKeys::new(storage.vault(), state.clone());
    let secrets = Secrets::new(storage.vault(), state.clone());
    let projects = Projects::start(state.clone())?;
    let always = AlwaysGrants::load(state, storage.grant_seal()?)?;

    let pool = Pool::builder().key_vault(Arc::new(keys.clone())).build();
    let tunnels = Tunnels::new(pool.clone());
    let terminals = Terminals::new(journal.clone());
    let runbooks = Runbooks::new(projects.clone(), journal.clone(), terminals.clone());
    let kube = Kube::new(pool.clone(), projects.clone(), journal.clone());

    let mut registry = Registry::new();
    register_hosts(&mut registry, &pool, &projects, &tunnels)?;
    kube.register(&mut registry)?;
    register_sessions(&mut registry, &pool, &projects, &terminals)?;
    register_records(&mut registry, &journal, &projects, &always)?;
    register_keys(&mut registry, &keys, &pool)?;
    register_secrets(&mut registry, &secrets)?;
    runbooks.register(&mut registry)?;

    let syncer = syncer(&storage, &projects, &journal)?;
    syncer.register(&mut registry)?;

    let extensions = Extensions::open(
        &data,
        &storage.state_path(),
        Runtime::new()?,
        Remote::official()?,
    )?
    .with_secrets(Arc::new(secrets))
    .with_projects(Arc::new(projects.clone()));
    extensions.register(&mut registry)?;
    load_extensions(&extensions).await?;

    let core = Arc::new(Core::new(
        registry,
        journal.clone(),
        Broker::with_always(always),
    ));
    extensions.connect(
        Arc::new(CoreCaller::new(&core)),
        Arc::new(CoreScreen::new(&core)),
    );
    runbooks.connect(&core);
    let _feed = EventFeed::start(&extensions, &journal, &projects);
    let server = Server::bind(&endpoint, core, apps()?)?;
    let _sync: SyncLoop = syncer.start(SYNC_EVERY);

    eprintln!(
        "synt-core {VERSION} listening on {}",
        endpoint.address().to_string_lossy()
    );

    let idle = IdleWatch::new(arguments.idle)
        .watch(terminals.clone())
        .watch(tunnels)
        .watch(runbooks.runs())
        .watch(kube);
    serve(server, idle, terminals).await;

    Ok(())
}

fn register_hosts(
    registry: &mut Registry,
    pool: &Pool,
    projects: &Projects,
    tunnels: &Tunnels,
) -> Result<(), ActionError> {
    registry.register(Ping)?;
    registry.register(SshExec::new(pool.clone(), projects.clone()))?;
    registry.register(SshWarm::new(pool.clone(), projects.clone()))?;
    registry.register(SshConnections::new(pool.clone()))?;

    registry.register(TunnelOpen::new(tunnels.clone(), projects.clone()))?;
    registry.register(TunnelList::new(tunnels.clone()))?;
    registry.register(TunnelClose::new(tunnels.clone()))?;

    Files::new(pool.clone(), projects.clone()).register(registry)
}

fn register_sessions(
    registry: &mut Registry,
    pool: &Pool,
    projects: &Projects,
    terminals: &Terminals,
) -> Result<(), ActionError> {
    registry.register(SshSessionOpen::new(
        pool.clone(),
        terminals.clone(),
        projects.clone(),
    ))?;
    registry.register(LocalSessionOpen::new(terminals.clone()))?;
    registry.register(SessionAttach::new(terminals.clone()))?;
    registry.register(SessionResize::new(terminals.clone()))?;
    registry.register(SessionClose::new(terminals.clone()))?;
    registry.register(SessionList::new(terminals.clone()))?;
    registry.register(SessionBlocks::new(terminals.clone()))?;
    registry.register(SessionIntegration)
}

fn register_records(
    registry: &mut Registry,
    journal: &JournalWriter,
    projects: &Projects,
    always: &AlwaysGrants,
) -> Result<(), ActionError> {
    registry.register(JournalPage::new(journal.clone()))?;
    registry.register(JournalVerify::new(journal.clone()))?;

    registry.register(ProjectOpen::new(projects.clone()))?;
    registry.register(ProjectClose::new(projects.clone()))?;
    registry.register(ProjectList::new(projects.clone()))?;
    registry.register(ResourceList::new(projects.clone()))?;
    registry.register(ResourceGet::new(projects.clone()))?;
    registry.register(ResourcePut::new(projects.clone()))?;
    registry.register(ResourceRemove::new(projects.clone()))?;

    registry.register(GrantList::new(always.clone()))?;
    registry.register(GrantRevoke::new(always.clone(), journal.clone()))
}

fn register_keys(registry: &mut Registry, keys: &SshKeys, pool: &Pool) -> Result<(), ActionError> {
    registry.register(KeyImport::new(keys.clone()))?;
    registry.register(KeyGenerate::new(keys.clone()))?;
    registry.register(KeyList::new(keys.clone()))?;
    registry.register(KeyRemove::new(keys.clone(), pool.clone()))
}

fn register_secrets(registry: &mut Registry, secrets: &Secrets) -> Result<(), ActionError> {
    registry.register(SecretPut::new(secrets.clone()))?;
    registry.register(SecretList::new(secrets.clone()))?;
    registry.register(SecretRemove::new(secrets.clone()))
}

fn syncer(
    storage: &Storage,
    projects: &Projects,
    journal: &JournalWriter,
) -> Result<Syncer, Box<dyn Error>> {
    let syncer = Syncer::builder(
        &storage.state_path(),
        storage.vault(),
        Arc::new(projects.clone()),
    )
    .journal(journal.clone())
    .open()?;

    Ok(syncer)
}

async fn serve(server: Server, idle: IdleWatch, terminals: Terminals) {
    let idle = idle.watch(server.activity());

    let stopping = async {
        tokio::select! {
            () = shutdown_signal() => {}
            () = idle.idle() => eprintln!("synt-core: idle, stopping"),
        }
        terminals.close_all().await;
    };

    server.run(stopping).await;
}

async fn load_extensions(extensions: &Extensions) -> Result<(), Box<dyn Error>> {
    let extensions = extensions.clone();
    let loaded = task::spawn_blocking(move || extensions.load_enabled()).await??;

    if loaded > 0 {
        eprintln!("loaded {loaded} extension(s)");
    }

    Ok(())
}

fn journal(data: &Path, weak_vault: WeakVault) -> Result<(Journal, Storage), Box<dyn Error>> {
    let vault = open_vault(data, weak_vault)?;
    let place = vault.place();
    eprintln!("journal keys are kept in {place}");

    if let Some(weakness) = place.weakness() {
        eprintln!("warning: {weakness}");
        eprintln!("warning: {}", weak_vault_reason(weak_vault));
    }

    let storage = Storage::prepare(data, vault)?;
    let journal = storage.journal()?;

    eprintln!(
        "journal {} at {}, last event {}",
        journal.id(),
        journal.path().display(),
        journal.anchors().head.seq
    );

    Ok((journal, storage))
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
