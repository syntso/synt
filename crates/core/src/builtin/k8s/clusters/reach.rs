use synt_actions::{ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::ClusterSpec;
use synt_ssh::Target;

use super::ClusterKey;
use crate::CoreError;
use crate::builtin::ssh::is_root;
use crate::projects::{Projects, Resolved};

#[derive(Debug, Clone)]
pub(crate) struct Reach {
    spec: ClusterSpec,
    via: Option<Target>,
    destination: String,
}

impl Reach {
    pub(crate) fn resolve(projects: &Projects, target: &ResourceRef) -> Result<Self, CoreError> {
        let Resolved::Cluster { spec, via } = projects.resolve_cluster(target.as_str())? else {
            return Err(CoreError::NotAClusterTarget(target.clone()));
        };

        let place = spec.place().map_err(CoreError::KubePlace)?;

        let through = via
            .as_ref()
            .map(|via| format!(" via {}", via.route()))
            .unwrap_or_default();
        let destination = format!("{} = {place}{through}", projects.canonical(target.as_str()));

        Ok(Self {
            spec,
            via,
            destination,
        })
    }

    pub(crate) fn spec(&self) -> &ClusterSpec {
        &self.spec
    }

    pub(crate) fn via(&self) -> Option<&Target> {
        self.via.as_ref()
    }

    pub(crate) fn into_destination(self) -> String {
        self.destination
    }

    pub(crate) fn logs_in_as_root(&self) -> bool {
        self.via.as_ref().is_some_and(|via| is_root(via.user()))
    }

    pub(crate) fn check_admitted(&self, context: &Context) -> Result<(), CoreError> {
        let target = context.target().ok_or(ActionError::TargetRequired)?;

        if self.logs_in_as_root() && context.risk() != Risk::Privileged {
            return Err(CoreError::RootLoginNow(target.clone()));
        }

        if context.destination() != Some(self.destination.as_str()) {
            return Err(CoreError::DestinationChanged {
                target: target.clone(),
                destination: self.destination.clone(),
            });
        }

        Ok(())
    }

    pub(crate) fn key(&self) -> ClusterKey {
        ClusterKey {
            destination: self.destination.clone(),
            kubeconfig_modified: self.spec.kubeconfig.modified(),
        }
    }
}
