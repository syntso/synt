mod builder;
mod engine;
mod notes;
mod reports;
mod round;
mod sync_loop;

pub use builder::SyncerBuilder;
pub use sync_loop::SyncLoop;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use synt_platform::Vault;
use tokio::{task, time};

use crate::document::DocumentPath;
use crate::id::{MemberId, RevisionId};
use crate::provider::Place;
use crate::report::{Disabled, MembershipChange, RevisionInfo, RoundReport, StatusReport};
use crate::{OpenProjects, SyncError};
use engine::Engine;

#[derive(Clone)]
pub struct Syncer {
    engine: Arc<Engine>,
}

impl Syncer {
    pub fn builder(
        state: &Path,
        vault: Arc<dyn Vault>,
        projects: Arc<dyn OpenProjects>,
    ) -> SyncerBuilder {
        SyncerBuilder::new(state, vault, projects)
    }

    pub(crate) fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(engine),
        }
    }

    pub fn start(&self, every: Duration) -> SyncLoop {
        let syncer = self.clone();

        SyncLoop::new(tokio::spawn(async move {
            loop {
                syncer.sync_now().await;
                time::sleep(every).await;
            }
        }))
    }

    pub async fn sync_now(&self) -> Vec<RoundReport> {
        self.run(|engine| Ok(engine.round_all()))
            .await
            .unwrap_or_default()
    }

    pub async fn enable(&self, project: &str, place: Place) -> Result<StatusReport, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.enable(&project, &place))
            .await
    }

    pub async fn disable(&self, project: &str) -> Result<Disabled, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.disable(&project)).await
    }

    pub async fn status(&self, project: &str) -> Result<StatusReport, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.status(&project)).await
    }

    pub async fn share(
        &self,
        project: &str,
        member: MemberId,
    ) -> Result<MembershipChange, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.share(&project, &member))
            .await
    }

    pub async fn unshare(
        &self,
        project: &str,
        member: MemberId,
    ) -> Result<MembershipChange, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.unshare(&project, &member))
            .await
    }

    pub async fn history(
        &self,
        project: &str,
        path: DocumentPath,
    ) -> Result<Vec<RevisionInfo>, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.history_of(&project, &path))
            .await
    }

    pub async fn resolve(
        &self,
        project: &str,
        path: DocumentPath,
        revision: RevisionId,
    ) -> Result<RevisionInfo, SyncError> {
        let project = project.to_owned();

        self.run(move |engine| engine.resolve(&project, &path, &revision))
            .await
    }

    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Engine) -> Result<T, SyncError> + Send + 'static,
    ) -> Result<T, SyncError> {
        let engine = Arc::clone(&self.engine);

        task::spawn_blocking(move || work(&engine))
            .await
            .map_err(|_| SyncError::Stopped)?
    }
}
