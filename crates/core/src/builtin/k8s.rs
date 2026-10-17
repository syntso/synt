mod apply;
mod clusters;
mod contexts;
mod delete;
mod error;
mod events;
mod exec;
mod forwards;
mod get;
mod kind_facts;
mod list;
mod logs;
mod params;
mod rollout_restart;
mod scale;
mod streams;
mod watch;

use apply::K8sApply;
use clusters::{Clusters, Reach};
use contexts::K8sContexts;
use delete::K8sDelete;
use error::K8sError;
use events::K8sEvents;
use exec::K8sExec;
use forwards::{ForwardId, Forwards, K8sPortForward, K8sPortForwardClose, K8sPortForwardList};
use get::K8sGet;
use kind_facts::KindFacts;
use list::K8sList;
use logs::K8sLogs;
use params::ObjectParams;
use rollout_restart::K8sRolloutRestart;
use scale::K8sScale;
use streams::{K8sStreamClose, Streams};
use watch::K8sWatch;

use std::future::Future;
use std::sync::Arc;

use synt_actions::{ActionError, ActionGroup, Context, Registry, Risk};
use synt_journal::{JournalWriter, ResourceRef};
use synt_kube::{Cluster, ContextInfo, Kind, KindName, KubeError};
use synt_ssh::Pool;
use tokio::runtime::Handle;
use tokio::task;

use crate::blocking::blocking;
use crate::projects::Projects;

#[derive(Debug, Clone)]
pub struct K8sActions {
    projects: Projects,
    clusters: Clusters,
    forwards: Forwards,
    streams: Streams,
}

impl K8sActions {
    pub fn new(pool: Pool, projects: Projects, journal: JournalWriter) -> Self {
        Self {
            projects,
            clusters: Clusters::new(pool),
            forwards: Forwards::default(),
            streams: Streams::new(journal),
        }
    }

    pub(crate) fn is_busy(&self) -> bool {
        !self.forwards.is_empty() || !self.streams.is_empty()
    }

    fn forwards(&self) -> &Forwards {
        &self.forwards
    }

    fn streams(&self) -> &Streams {
        &self.streams
    }

    async fn contexts(&self) -> Result<Vec<ContextInfo>, K8sError> {
        let source = self.projects.personal_kubeconfig().clone();

        let reading = task::spawn_blocking(move || source.contexts());

        Ok(reading.await.map_err(|_| K8sError::KubeconfigStopped)??)
    }

    fn reach(&self, target: Option<&ResourceRef>) -> Result<Reach, K8sError> {
        let target = target.ok_or(ActionError::TargetRequired)?;

        Reach::resolve(&self.projects, target)
    }

    fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        self.reach(target).ok().map(Reach::into_destination)
    }

    fn risk(&self, target: Option<&ResourceRef>, declared: Risk) -> Risk {
        match self.reach(target) {
            Ok(reach) if !reach.logs_in_as_root() => declared,
            _ => Risk::Privileged,
        }
    }

    fn inspect<T, Work, Running>(
        &self,
        target: Option<&ResourceRef>,
        work: Work,
    ) -> Result<Option<T>, K8sError>
    where
        Work: FnOnce(Arc<Cluster>) -> Running,
        Running: Future<Output = Result<T, KubeError>>,
    {
        let Some(cluster) = self.inspected_cluster(target)? else {
            return Ok(None);
        };

        let inspected = Self::block_on(work(cluster))??;

        Ok(Some(inspected))
    }

    fn kind_facts(
        &self,
        target: Option<&ResourceRef>,
        name: &KindName,
    ) -> Result<Option<KindFacts>, K8sError> {
        let kind = self.inspect(target, |cluster| async move { cluster.kind(name).await })?;

        Ok(kind.as_ref().map(KindFacts::from))
    }

    async fn cluster(&self, context: &Context) -> Result<Arc<Cluster>, K8sError> {
        let target = context
            .target()
            .cloned()
            .ok_or(ActionError::TargetRequired)?;
        let projects = self.projects.clone();

        let reach =
            blocking::<_, K8sError>(move || Ok(Reach::resolve(&projects, &target))).await??;
        reach.check_admitted(context)?;

        self.clusters.connect(&reach).await
    }

    async fn kind(
        &self,
        context: &Context,
        cluster: &Cluster,
        name: &KindName,
    ) -> Result<Kind, K8sError> {
        let kind = cluster.kind(name).await?;

        if kind.is_secret() && context.risk() != Risk::Privileged {
            return Err(K8sError::SecretsNow(name.to_string()));
        }

        Ok(kind)
    }

    fn inspected_cluster(
        &self,
        target: Option<&ResourceRef>,
    ) -> Result<Option<Arc<Cluster>>, K8sError> {
        let reach = self.reach(target)?;

        if reach.logs_in_as_root() {
            return Ok(self.clusters.cached(&reach));
        }

        Self::block_on(self.clusters.connect(&reach))?.map(Some)
    }

    fn block_on<T>(future: impl Future<Output = T>) -> Result<T, K8sError> {
        let runtime = Handle::try_current().map_err(|_| K8sError::OutsideCore)?;

        Ok(runtime.block_on(future))
    }
}

impl ActionGroup for K8sActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(K8sContexts::new(self.clone()))?;
        registry.register(K8sList::new(self.clone()))?;
        registry.register(K8sGet::new(self.clone()))?;
        registry.register(K8sEvents::new(self.clone()))?;
        registry.register(K8sLogs::new(self.clone()))?;
        registry.register(K8sWatch::new(self.clone()))?;
        registry.register(K8sStreamClose::new(self.clone()))?;
        registry.register(K8sApply::new(self.clone()))?;
        registry.register(K8sScale::new(self.clone()))?;
        registry.register(K8sRolloutRestart::new(self.clone()))?;
        registry.register(K8sDelete::new(self.clone()))?;
        registry.register(K8sExec::new(self.clone()))?;
        registry.register(K8sPortForward::new(self.clone()))?;
        registry.register(K8sPortForwardList::new(self.clone()))?;
        registry.register(K8sPortForwardClose::new(self.clone()))
    }
}
