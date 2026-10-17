mod error;
mod explain;
mod open;
mod ping;
mod query;
mod recorded;
mod schema;
mod tables;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use synt_actions::{ActionError, Context, Registry, Risk};
use synt_db::{Database, DbError, Effect, Engine, Server};
use synt_journal::ResourceRef;
use synt_ssh::{Destination, Pool, Target, Tunnels};
use tokio::sync::Semaphore;
use tokio::time;
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

use self::error::DbActionError;
use self::explain::DbExplain;
use self::open::OpenDatabase;
use self::ping::DbPing;
use self::query::DbQuery;
use self::schema::DbSchema;
use self::tables::DbTables;
use crate::blocking::blocking;
use crate::builtin::ssh::is_root;
use crate::projects::{Projects, ResolvedDatabase};
use crate::secrets::Secrets;

const CONCURRENT_CALLS: usize = 8;

#[derive(Debug, Clone)]
pub struct Databases {
    projects: Projects,
    secrets: Secrets,
    tunnels: Tunnels,
    calls: Arc<Semaphore>,
}

impl Databases {
    pub fn new(pool: Pool, projects: Projects, secrets: Secrets) -> Self {
        Self {
            projects,
            secrets,
            tunnels: Tunnels::new(pool),
            calls: Arc::new(Semaphore::new(CONCURRENT_CALLS)),
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(DbQuery::new(self.clone()))?;
        registry.register(DbExplain::new(self.clone()))?;
        registry.register(DbSchema::new(self.clone()))?;
        registry.register(DbTables::new(self.clone()))?;
        registry.register(DbPing::new(self.clone()))
    }

    fn check_target(&self, target: Option<&ResourceRef>) -> Result<Engine, DbActionError> {
        let target = target.ok_or(ActionError::TargetRequired)?;

        let resolved = self
            .projects
            .database(target.as_str())
            .map_err(DbActionError::Target)?;

        Ok(resolved.resource.engine)
    }

    fn engine(&self, target: Option<&ResourceRef>) -> Option<Engine> {
        self.resolve(target)
            .map(|resolved| resolved.resource.engine)
    }

    fn current_engine(&self, context: &Context) -> Result<Engine, DbActionError> {
        self.engine(context.target())
            .ok_or(DbActionError::NotADatabase)
    }

    fn risk(&self, target: Option<&ResourceRef>, effect: Effect) -> Risk {
        let Some(resolved) = self.resolve(target) else {
            return Risk::Privileged;
        };

        if resolved.via.as_ref().is_some_and(|via| is_root(via.user())) {
            return Risk::Privileged;
        }

        match effect {
            Effect::Read => Risk::Read,
            Effect::Write => Risk::Write,
            Effect::Privileged => Risk::Privileged,
        }
    }

    fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        let target = target?;
        let resolved = self.resolve(Some(target))?;

        Some(self.projects.destination(target.as_str(), &resolved))
    }

    async fn with<T>(
        &self,
        context: &Context,
        limit: Duration,
        work: impl AsyncFnOnce(&mut Database) -> Result<T, DbError>,
    ) -> Result<T, DbActionError> {
        let _permit = self
            .calls
            .acquire()
            .await
            .map_err(|_| DbActionError::ShuttingDown)?;

        let running = async {
            let open = self.open(context).await?;
            Ok(open.serve(work).await?)
        };

        time::timeout(limit, running)
            .await
            .map_err(|_| DbActionError::Timeout(limit))?
    }

    async fn open(&self, context: &Context) -> Result<OpenDatabase, DbActionError> {
        let target = context.target().ok_or(ActionError::TargetRequired)?;
        let resolved = self.reach(context, target).await?;
        let server = self.server(&resolved).await?;

        let Some(via) = &resolved.via else {
            let database = Database::connect(&server).await?;
            return Ok(OpenDatabase::new(database, None));
        };

        let stop = CancellationToken::new();
        let local = self.tunnel(via, &resolved, stop.clone()).await?;
        let database = Database::connect(&server.reached_at(local)).await?;

        Ok(OpenDatabase::new(database, Some(stop.drop_guard())))
    }

    async fn reach(
        &self,
        context: &Context,
        target: &ResourceRef,
    ) -> Result<ResolvedDatabase, DbActionError> {
        let projects = self.projects.clone();
        let text = target.as_str().to_owned();
        let resolved = blocking::<_, DbActionError>(move || projects.database(&text)).await?;

        if resolved.via.as_ref().is_some_and(|via| is_root(via.user()))
            && context.risk() != Risk::Privileged
        {
            return Err(DbActionError::RootLogin {
                target: target.to_string(),
            });
        }

        let reached = self.projects.destination(target.as_str(), &resolved);

        if context.destination() != Some(reached.as_str()) {
            return Err(DbActionError::Moved {
                target: target.to_string(),
                reached,
            });
        }

        Ok(resolved)
    }

    async fn server(&self, resolved: &ResolvedDatabase) -> Result<Server, DbActionError> {
        let password = self.password(resolved).await?;

        Ok(resolved.resource.server(password)?)
    }

    async fn password(
        &self,
        resolved: &ResolvedDatabase,
    ) -> Result<Option<Zeroizing<String>>, DbActionError> {
        let Some(name) = resolved.resource.password_name()? else {
            return Ok(None);
        };

        let secrets = self.secrets.clone();
        let used_for = format!("the database {resolved}");
        let password = blocking::<_, DbActionError>(move || secrets.read_text_for(&name, &used_for)).await?;

        Ok(Some(password))
    }

    async fn tunnel(
        &self,
        via: &Target,
        resolved: &ResolvedDatabase,
        stop: CancellationToken,
    ) -> Result<SocketAddr, DbActionError> {
        let host = resolved.resource.host.clone().unwrap_or_default();
        let destination = Destination::new(host, resolved.resource.port());

        let opened = self.tunnels.open(via, destination, None, stop).await?;

        Ok(SocketAddr::from((Ipv4Addr::LOCALHOST, opened.local_port)))
    }

    fn resolve(&self, target: Option<&ResourceRef>) -> Option<ResolvedDatabase> {
        self.projects.database(target?.as_str()).ok()
    }
}
