use synt_actions::{ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::ClusterSpec;
use synt_ssh::Target;

use crate::CoreError;
use crate::builtin::ssh::is_root;
use crate::projects::{Projects, Resolved, login_label};

#[derive(Debug, Clone)]
pub(crate) struct Reach {
    spec: ClusterSpec,
    via: Option<Target>,
    destination: String,
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

    pub(crate) fn check_admitted(&self, context: &Context) -> Result<(), ActionError> {
        let target = context.target().ok_or(ActionError::TargetRequired)?;

        if self.logs_in_as_root() && context.risk() != Risk::Privileged {
            return Err(ActionError::Failed(format!(
                "{target} is now reached through a root login, which the call was not admitted for; \
                 run it again so it is asked as privileged"
            )));
        }

        if context.destination() != Some(self.destination.as_str()) {
            return Err(ActionError::Failed(format!(
                "{target} now leads to {}, which is not what the call was admitted for; \
                 run it again so it is checked for the new place",
                self.destination
            )));
        }

        Ok(())
    }

    pub(crate) fn key(&self) -> String {
        format!(
            "{} @ {:?}",
            self.destination,
            self.spec.kubeconfig.modified()
        )
    }
}
