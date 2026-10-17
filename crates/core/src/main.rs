use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use synt_actions::{ActionError, Registry};
use synt_core::{
    AlwaysGrants, Arguments, Broker, Core, CoreCaller, CoreScreen, Databases, EventFeed,
    ExtensionMerger, Files, GrantActions, IdleWatch, JournalActions, KeyActions, Kube,
    LoginCommand, Ping, ProjectActions, Projects, ResourceActions, Runbooks, SecretActions,
    Secrets, Server, SessionActions, SshActions, SshKeys, Storage, Terminals, TunnelActions,
    VERSION,
};
use synt_extensions::{Extensions, Remote, Runtime};
use synt_journal::{Journal, JournalWriter};
use synt_platform::{AppCheck, WEAK_VAULT_FLAG, WeakVault, open_vault, shutdown_signal};
use synt_ssh::{Pool, Tunnels};
use synt_sync::{SyncLoop, Syncer};
use tokio::task;

const SYNC_EVERY: Duration = Duration::from_secs(10);

struct Builtins {
    journal: JournalWriter,
    projects: Projects,
    keys: SshKeys,
    secrets: Secrets,
    always: AlwaysGrants,
    pool: Pool,
    tunnels: Tunnels,
    terminals: Terminals,
    runbooks: Runbooks,
    kube: Kube,
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

    let arguments = Arguments::parse(env::args_os().skip(1))?;
    let endpoint = arguments.endpoint()?;
    let data = arguments.data()?;

    let (journal, storage) = journal(&data, arguments.weak_vault())?;
    let builtins = builtins(&storage, JournalWriter::spawn(journal)?)?;

    let mut registry = Registry::new();
    register(&mut registry, &builtins)?;

    let extensions = extensions(&data, &storage, &builtins)?;
    let syncer = syncer(&storage, &builtins.projects, &builtins.journal, &extensions)?;
    syncer.register(&mut registry)?;
    extensions.register(&mut registry)?;
    load_extensions(&extensions).await?;

    let core = connect(registry, &builtins, &extensions);
    let _feed = EventFeed::start(&extensions, &builtins.journal, &builtins.projects);
    let server = Server::bind(&endpoint, core, apps()?)?;
    let _sync: SyncLoop = syncer.start(SYNC_EVERY);

    eprintln!(
        "synt-core {VERSION} listening on {}",
        endpoint.address().to_string_lossy()
    );

    let idle = IdleWatch::new(arguments.idle())
        .watch(builtins.terminals.clone())
        .watch(builtins.tunnels)
        .watch(builtins.runbooks.runs())
        .watch(builtins.kube);
    serve(server, idle, builtins.terminals).await;

    Ok(())
}

fn builtins(storage: &Storage, journal: JournalWriter) -> Result<Builtins, Box<dyn Error>> {
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

    Ok(Builtins {
        journal,
        projects,
        keys,
        secrets,
        always,
        pool,
        tunnels,
        terminals,
        runbooks,
        kube,
    })
}

fn register(registry: &mut Registry, builtins: &Builtins) -> Result<(), ActionError> {
    let Builtins {
        journal,
        projects,
        keys,
        secrets,
        always,
        pool,
        tunnels,
        terminals,
        runbooks,
        kube,
    } = builtins;

    registry.register(Ping)?;
    SshActions::new(pool.clone(), projects.clone()).register(registry)?;
    TunnelActions::new(tunnels.clone(), projects.clone()).register(registry)?;
    Files::new(pool.clone(), projects.clone()).register(registry)?;
    kube.register(registry)?;
    SessionActions::new(pool.clone(), projects.clone(), terminals.clone()).register(registry)?;
    JournalActions::new(journal.clone()).register(registry)?;
    ProjectActions::new(projects.clone()).register(registry)?;
    ResourceActions::new(projects.clone()).register(registry)?;
    GrantActions::new(always.clone(), journal.clone()).register(registry)?;
    KeyActions::new(keys.clone(), pool.clone()).register(registry)?;
    SecretActions::new(secrets.clone()).register(registry)?;
    Databases::new(pool.clone(), projects.clone(), secrets.clone()).register(registry)?;
    runbooks.register(registry)
}

fn extensions(
    data: &Path,
    storage: &Storage,
    builtins: &Builtins,
) -> Result<Extensions, Box<dyn Error>> {
    let extensions = Extensions::open(
        data,
        &storage.state_path(),
        Runtime::new()?,
        Remote::official()?,
    )?
    .with_secrets(Arc::new(builtins.secrets.clone()))
    .with_projects(Arc::new(builtins.projects.clone()));

    Ok(extensions)
}

fn connect(registry: Registry, builtins: &Builtins, extensions: &Extensions) -> Arc<Core> {
    let broker = Broker::with_always(builtins.always.clone());
    let core = Arc::new(Core::new(registry, builtins.journal.clone(), broker));

    extensions.connect(
        Arc::new(CoreCaller::new(&core)),
        Arc::new(CoreScreen::new(&core)),
    );
    builtins.runbooks.connect(&core);

    core
}

fn syncer(
    storage: &Storage,
    projects: &Projects,
    journal: &JournalWriter,
    extensions: &Extensions,
) -> Result<Syncer, Box<dyn Error>> {
    let syncer = Syncer::builder(
        &storage.state_path(),
        storage.vault(),
        Arc::new(projects.clone()),
    )
    .journal(journal.clone())
    .merger(Arc::new(ExtensionMerger::new(extensions.clone())))
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

fn weak_vault_reason(weak_vault: WeakVault) -> String {
    match weak_vault {
        WeakVault::Allow => format!("allowed by {WEAK_VAULT_FLAG}, which is for development only"),
        WeakVault::Refuse => format!(
            "allowed only because this is a debug build; a release build refuses this \
             without {WEAK_VAULT_FLAG}"
        ),
    }
}
