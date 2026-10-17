mod db;
mod grant;
mod hold;
mod journal;
mod k8s;
mod key;
mod ping;
mod project;
mod resource;
mod runbook;
mod secret;
mod session;
mod sftp;
mod ssh;
mod sync;
mod tunnel;

pub use db::DbActions;
pub use grant::{GrantActions, GrantList, GrantRevoke};
pub use journal::{JournalActions, JournalPage, JournalVerify};
pub use k8s::K8sActions;
pub use key::{KeyActions, KeyGenerate, KeyImport, KeyList, KeyRemove};
pub use ping::{Ping, Pong};
pub use project::{ProjectActions, ProjectClose, ProjectList, ProjectOpen};
pub use resource::{ResourceActions, ResourceGet, ResourceList, ResourcePut, ResourceRemove};
pub use runbook::RunbookActions;
pub use secret::{SecretActions, SecretList, SecretPut, SecretRemove};
pub use session::{
    LocalSessionOpen, SessionActions, SessionAttach, SessionBlocks, SessionClose,
    SessionIntegration, SessionList, SessionResize, SshSessionOpen,
};
pub use sftp::SftpActions;
pub use ssh::{SshActions, SshConnections, SshExec, SshWarm};
pub use sync::SyncActions;
pub use tunnel::{TunnelActions, TunnelClose, TunnelList, TunnelOpen};

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use synt_actions::{ActionGroup, Registry};
use synt_extensions::{Extensions, Remote, Runtime};
use synt_journal::JournalWriter;
use synt_platform::shutdown_signal;
use synt_ssh::{Pool, Tunnels};
use synt_sync::{SyncLoop, Syncer};

use crate::CoreError;
use crate::blocking::blocking;
use crate::broker::Broker;
use crate::core::Core;
use crate::extensions::{CoreCaller, CoreScreen, EventFeed, ExtensionMerger};
use crate::grants::AlwaysGrants;
use crate::idle::IdleWatch;
use crate::projects::Projects;
use crate::runbooks::Runbooks;
use crate::secrets::Secrets;
use crate::server::Server;
use crate::ssh_keys::SshKeys;
use crate::storage::Storage;
use crate::terminals::Terminals;

const SYNC_EVERY: Duration = Duration::from_secs(10);

pub struct Builtins {
    journal: JournalWriter,
    projects: Projects,
    keys: SshKeys,
    secrets: Secrets,
    always: AlwaysGrants,
    pool: Pool,
    tunnels: Tunnels,
    terminals: Terminals,
    runbooks: Runbooks,
    kube: K8sActions,
    extensions: Extensions,
    syncer: Syncer,
}

impl Builtins {
    pub fn start(data: &Path, storage: &Storage) -> Result<Self, CoreError> {
        let journal = storage.open_journal()?;
        let state = storage.state()?;

        let keys = SshKeys::new(storage.vault(), state.clone());
        let secrets = Secrets::new(storage.vault(), state.clone());
        let projects = Projects::start(state.clone())?;
        let always = AlwaysGrants::load(state, storage.grant_seal()?)?;

        let pool = Pool::builder().key_vault(Arc::new(keys.clone())).build();
        let terminals = Terminals::new(journal.clone());
        let runbooks = Runbooks::new(projects.clone(), journal.clone(), terminals.clone());
        let kube = K8sActions::new(pool.clone(), projects.clone(), journal.clone());

        let extensions = Self::open_extensions(data, storage, &secrets, &projects)?;
        let syncer = Self::open_syncer(storage, &projects, &journal, &extensions)?;

        Ok(Self {
            tunnels: Tunnels::new(pool.clone()),
            journal,
            projects,
            keys,
            secrets,
            always,
            pool,
            terminals,
            runbooks,
            kube,
            extensions,
            syncer,
        })
    }

    pub async fn connect(&self) -> Result<Arc<Core>, CoreError> {
        let mut registry = Registry::new();

        for group in self.groups() {
            group.register(&mut registry)?;
        }

        self.load_extensions().await?;

        let broker = Broker::with_always(self.always.clone());
        let core = Arc::new(Core::new(registry, self.journal.clone(), broker));

        self.extensions.connect(
            Arc::new(CoreCaller::new(&core)),
            Arc::new(CoreScreen::new(&core)),
        );
        self.runbooks.connect(&core)?;

        Ok(core)
    }

    pub async fn serve(self, server: Server, idle: Option<Duration>) {
        let _feed = EventFeed::start(&self.extensions, &self.journal, &self.projects);
        let _sync: SyncLoop = self.syncer.start(SYNC_EVERY);

        let idle = IdleWatch::new(idle)
            .watch(self.terminals.clone())
            .watch(self.tunnels)
            .watch(self.runbooks.runs())
            .watch(self.kube)
            .watch(server.activity());

        let terminals = self.terminals;
        let stopping = async {
            tokio::select! {
                () = shutdown_signal() => {}
                () = idle.idle() => eprintln!("synt-core: idle, stopping"),
            }
            terminals.close_all().await;
        };

        server.run(stopping).await;
    }

    fn groups(&self) -> Vec<Box<dyn ActionGroup>> {
        let pool = &self.pool;
        let projects = &self.projects;

        vec![
            Box::new(Ping),
            Box::new(SshActions::new(pool.clone(), projects.clone())),
            Box::new(TunnelActions::new(self.tunnels.clone(), projects.clone())),
            Box::new(SftpActions::new(pool.clone(), projects.clone())),
            Box::new(self.kube.clone()),
            Box::new(SessionActions::new(
                pool.clone(),
                projects.clone(),
                self.terminals.clone(),
            )),
            Box::new(JournalActions::new(self.journal.clone())),
            Box::new(ProjectActions::new(projects.clone())),
            Box::new(ResourceActions::new(projects.clone())),
            Box::new(GrantActions::new(self.always.clone(), self.journal.clone())),
            Box::new(KeyActions::new(self.keys.clone(), pool.clone())),
            Box::new(SecretActions::new(self.secrets.clone())),
            Box::new(DbActions::new(
                pool.clone(),
                projects.clone(),
                self.secrets.clone(),
            )),
            Box::new(RunbookActions::new(self.runbooks.clone())),
            Box::new(SyncActions::new(self.syncer.clone())),
            Box::new(self.extensions.clone()),
        ]
    }

    fn open_extensions(
        data: &Path,
        storage: &Storage,
        secrets: &Secrets,
        projects: &Projects,
    ) -> Result<Extensions, CoreError> {
        let extensions = Extensions::open(
            data,
            &storage.state_path(),
            Runtime::new()?,
            Remote::official()?,
        )?
        .with_secrets(Arc::new(secrets.clone()))
        .with_projects(Arc::new(projects.clone()));

        Ok(extensions)
    }

    fn open_syncer(
        storage: &Storage,
        projects: &Projects,
        journal: &JournalWriter,
        extensions: &Extensions,
    ) -> Result<Syncer, CoreError> {
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

    async fn load_extensions(&self) -> Result<(), CoreError> {
        let extensions = self.extensions.clone();
        let loaded = blocking::<_, CoreError>(move || Ok(extensions.load_enabled()?)).await?;

        if loaded > 0 {
            eprintln!("loaded {loaded} extension(s)");
        }

        Ok(())
    }
}
