use synt_journal::ResourceRef;
use synt_kube::ClusterSpec;
use synt_ssh::Target;

use crate::CoreError;
use crate::builtin::ssh::is_root;
use crate::projects::{Projects, Resolved, login_label};

#[derive(Debug, Clone)]
pub(crate) struct Reach {
    pub(crate) spec: ClusterSpec,
    pub(crate) via: Option<Target>,
    pub(crate) destination: String,
}

impl Reach {
    pub(crate) fn resolve(projects: &Projects, target: &ResourceRef) -> Result<Self, CoreError> {
        let Resolved::Cluster { spec, via } = projects.resolve_cluster(target.as_str())? else {
            return Err(CoreError::Project(format!("{target} is not a cluster")));
        };

        let place = spec
            .place()
            .map_err(|error| CoreError::Project(error.to_string()))?;

        let through = via
            .as_ref()
            .map(|via| format!(" via {}", login_label(via)))
            .unwrap_or_default();
        let destination = format!("{} = {place}{through}", projects.canonical(target.as_str()));

        Ok(Self {
            spec,
            via,
            destination,
        })
    }

    pub(crate) fn logs_in_as_root(&self) -> bool {
        self.via.as_ref().is_some_and(|via| is_root(via.user()))
    }

    pub(crate) fn key(&self) -> String {
        format!(
            "{} @ {:?}",
            self.destination,
            self.spec.kubeconfig.modified()
        )
    }
}
