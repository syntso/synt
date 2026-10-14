use std::future::Future;
use std::sync::Arc;

use synt_actions::{ActionError, Context, Registry, Risk};
use synt_journal::{JournalWriter, ResourceRef};
use synt_kube::{Cluster, ContextInfo, Kind};
use synt_ssh::Pool;
use tokio::runtime::Handle;
use tokio::task;

use super::failure::failed;
use super::forwards::Forwards;
use super::streams::Streams;
use super::{
    Clusters, K8sApply, K8sContexts, K8sDelete, K8sEvents, K8sExec, K8sGet, K8sList, K8sLogs,
    K8sPortForward, K8sPortForwardClose, K8sPortForwardList, K8sRolloutRestart, K8sScale,
    K8sStreamClose, K8sWatch, KindFacts, Reach,
};
use crate::builtin::blocking::blocking;
use crate::projects::Projects;

#[derive(Debug, Clone)]
pub struct Kube {
    projects: Projects,
    clusters: Clusters,
    forwards: Forwards,
    streams: Streams,
}

impl Kube {
    pub fn new(pool: Pool, projects: Projects, journal: JournalWriter) -> Self {
        Self {
            projects,
            clusters: Clusters::new(pool),
            forwards: Forwards::default(),
            streams: Streams::new(journal),
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
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

    pub(crate) fn is_busy(&self) -> bool {
        !self.forwards.is_empty() || !self.streams.is_empty()
    }

    pub(super) fn forwards(&self) -> &Forwards {
        &self.forwards
    }

    pub(super) fn streams(&self) -> &Streams {
        &self.streams
    }

    pub(super) async fn contexts(&self) -> Result<Vec<ContextInfo>, ActionError> {
        let source = self.projects.personal_kubeconfig().clone();

        task::spawn_blocking(move || source.contexts())
            .await
            .map_err(|_| ActionError::failed("reading the kubeconfig stopped unexpectedly"))?
            .map_err(failed)
    }

    pub(super) fn reach(&self, target: Option<&ResourceRef>) -> Result<Reach, ActionError> {
        let target = target.ok_or(ActionError::TargetRequired)?;

        Ok(Reach::resolve(&self.projects, target)?)
    }

    pub(super) fn destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        self.reach(target).ok().map(Reach::into_destination)
    }

    pub(super) fn risk(&self, target: Option<&ResourceRef>, declared: Risk) -> Risk {
        match self.reach(target) {
            Ok(reach) if !reach.logs_in_as_root() => declared,
            _ => Risk::Privileged,
        }
    }

    pub(super) fn inspected_cluster(
        &self,
        target: Option<&ResourceRef>,
    ) -> Result<Option<Arc<Cluster>>, ActionError> {
        let reach = self.reach(target)?;

        if reach.logs_in_as_root() {
            return Ok(self.clusters.cached(&reach));
        }

        wait_for(self.clusters.connect(&reach))?.map(Some)
    }

    pub(super) fn kind_facts(
        &self,
        target: Option<&ResourceRef>,
        kind: &str,
    ) -> Result<Option<KindFacts>, ActionError> {
        let Some(cluster) = self.inspected_cluster(target)? else {
            return Ok(None);
        };

        let kind = wait_for(cluster.kind(kind))?.map_err(failed)?;

        Ok(Some(KindFacts::from(&kind)))
    }

    pub(super) async fn cluster(&self, context: &Context) -> Result<Arc<Cluster>, ActionError> {
        let target = context
            .target()
            .cloned()
            .ok_or(ActionError::TargetRequired)?;
        let projects = self.projects.clone();

        let reach = blocking(move || Reach::resolve(&projects, &target)).await?;
        reach.check_admitted(context)?;

        self.clusters.connect(&reach).await
    }

    pub(super) async fn kind(
        &self,
        context: &Context,
        cluster: &Cluster,
        text: &str,
    ) -> Result<Kind, ActionError> {
        let kind = cluster.kind(text).await.map_err(failed)?;

        if kind.is_secret() && context.risk() != Risk::Privileged {
            return Err(ActionError::failed(format!(
                "{text} names secrets now, which was not known when the call was admitted; \
                 run it again so it is asked as privileged"
            )));
        }

        Ok(kind)
    }
}

pub(super) fn wait_for<T>(future: impl Future<Output = T>) -> Result<T, ActionError> {
    let runtime = Handle::try_current()
        .map_err(|_| ActionError::failed("the cluster can only be reached from inside the core"))?;

    Ok(runtime.block_on(future))
}
