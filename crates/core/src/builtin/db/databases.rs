use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use synt_actions::{ActionError, Context, Registry, Risk};
use synt_db::{Database, DbError, Effect, Engine};
use synt_journal::ResourceRef;
use synt_ssh::{Destination, Pool, Target, Tunnels};
use tokio::sync::Semaphore;
use tokio::time;
use tokio_util::sync::{CancellationToken, DropGuard};

use super::failure::failure;
use super::open_database::OpenDatabase;
use super::{DbExplain, DbPing, DbQuery, DbSchema, DbTables};
use crate::builtin::blocking::blocking;
use crate::builtin::ssh::{failure as ssh_failure, is_root};
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

    pub(super) fn check_target(&self, target: Option<&ResourceRef>) -> Result<Engine, ActionError> {
        let target = target.ok_or(ActionError::TargetRequired)?;

        self.projects
            .database(target.as_str())
            .map(|resolved| resolved.resource.engine)
            .map_err(|error| ActionError::InvalidParams(error.to_string()))
    }

    pub(super) fn engine(&self, target: Option<&ResourceRef>) -> Option<Engine> {
        self.resolve(target)
            .map(|resolved| resolved.resource.engine)
    }

    pub(super) fn risk(&self, target: Option<&ResourceRef>, effect: Effect) -> Risk {
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

    pub(super) fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        let target = target?;
        let resolved = self.resolve(Some(target))?;

        Some(
            self.projects
                .database_destination(target.as_str(), &resolved),
        )
    }

    pub(super) async fn with<T>(
        &self,
        context: &Context,
        limit: Duration,
        work: impl AsyncFnOnce(&mut Database) -> Result<T, DbError>,
    ) -> Result<T, ActionError> {
        let _permit = self
            .calls
            .acquire()
            .await
            .map_err(|_| ActionError::failed("the database actions are shutting down"))?;

        let running = async {
            let mut open = self.open(context).await?;
            let done = work(&mut open.database).await.map_err(failure);
            open.close().await;
            done
        };

        time::timeout(limit, running).await.map_err(|_| {
            ActionError::Failed(format!(
                "no answer within {} s, so the call was stopped",
                limit.as_secs()
            ))
        })?
    }

    async fn open(&self, context: &Context) -> Result<OpenDatabase, ActionError> {
        let target = context.target().ok_or(ActionError::TargetRequired)?;
        let resolved = self.reach(context, target).await?;
        let password = self.password(&resolved).await?;

        let server = resolved
            .resource
            .server(password)
            .map_err(ActionError::Failed)?;

        let Some(via) = &resolved.via else {
            let database = Database::connect(&server).await.map_err(failure)?;
            return Ok(OpenDatabase::new(database, None));
        };

        let (local, tunnel) = self.tunnel(via, &resolved).await?;
        let database = Database::connect(&server.reached_at(local))
            .await
            .map_err(failure)?;

        Ok(OpenDatabase::new(database, Some(tunnel)))
    }

    async fn reach(
        &self,
        context: &Context,
        target: &ResourceRef,
    ) -> Result<ResolvedDatabase, ActionError> {
        let projects = self.projects.clone();
        let text = target.as_str().to_owned();
        let resolved = blocking(move || projects.database(&text)).await?;

        if resolved.via.as_ref().is_some_and(|via| is_root(via.user()))
            && context.risk() != Risk::Privileged
        {
            return Err(ActionError::Failed(format!(
                "{target} is now reached through a root login, which the call was not \
                 admitted for; run it again so it is asked as privileged"
            )));
        }

        let reached = self
            .projects
            .database_destination(target.as_str(), &resolved);

        if context.destination() != Some(reached.as_str()) {
            return Err(ActionError::Failed(format!(
                "{target} now leads to {reached}, which is not what the call was admitted \
                 for; run it again so it is checked for the new place"
            )));
        }

        Ok(resolved)
    }

    async fn password(
        &self,
        resolved: &ResolvedDatabase,
    ) -> Result<Option<zeroize::Zeroizing<String>>, ActionError> {
        let Some(name) = resolved
            .resource
            .password_name()
            .map_err(ActionError::Failed)?
        else {
            return Ok(None);
        };

        let secrets = self.secrets.clone();

        blocking(move || secrets.read_text(&name)).await.map(Some)
    }

    async fn tunnel(
        &self,
        via: &Target,
        resolved: &ResolvedDatabase,
    ) -> Result<(SocketAddr, DropGuard), ActionError> {
        let host = resolved.resource.host.clone().unwrap_or_default();
        let destination = Destination::new(host, resolved.resource.port());
        let stop = CancellationToken::new();

        let opened = self
            .tunnels
            .open(via, destination, None, stop.clone())
            .await
            .map_err(|error| ssh_failure(&error))?;

        let local = SocketAddr::from((Ipv4Addr::LOCALHOST, opened.local_port));

        Ok((local, stop.drop_guard()))
    }

    fn resolve(&self, target: Option<&ResourceRef>) -> Option<ResolvedDatabase> {
        self.projects.database(target?.as_str()).ok()
    }
}
