use synt_actions::{ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Target;

use super::privilege::is_root;
use crate::blocking::blocking;
use crate::projects::{Projects, Resolved};

impl Projects {
    pub(crate) fn login_risk(&self, target: Option<&ResourceRef>, declared: Risk) -> Risk {
        if self.logs_in_as_root(target) {
            return Risk::Privileged;
        }

        declared
    }

    pub(crate) fn host_destination(&self, target: Option<&ResourceRef>) -> Option<String> {
        let target = target?;
        let resolved = self.resolve(target.as_str()).ok()?;

        if matches!(resolved, Resolved::Cluster { .. }) {
            return None;
        }

        Some(self.destination(target.as_str(), &resolved))
    }

    pub(crate) async fn reach_host(&self, context: &Context) -> Result<Target, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;
        let resolved = self.resolve_any(host.as_str()).await?;

        let target = match &resolved {
            Resolved::Host(target) => target,
            Resolved::Tunnel { .. } => {
                return Err(ActionError::failed(format!(
                    "{host} is a tunnel, not a host; open it with tunnel.open"
                )));
            }
            Resolved::Cluster { .. } => {
                return Err(ActionError::failed(format!(
                    "{host} is a cluster, not a host; reach it with the k8s actions"
                )));
            }
        };

        self.check_reach(context, &resolved)?;

        Ok(target.clone())
    }

    pub(crate) fn check_reach(
        &self,
        context: &Context,
        resolved: &Resolved,
    ) -> Result<(), ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;

        if resolved.logs_in_as_root() && context.risk() != Risk::Privileged {
            return Err(hidden_root(host));
        }

        let reached = self.destination(host.as_str(), resolved);

        if context.destination() != Some(reached.as_str()) {
            return Err(moved(host, &reached));
        }

        Ok(())
    }

    pub(crate) async fn resolve_any(&self, spec: &str) -> Result<Resolved, ActionError> {
        let projects = self.clone();
        let spec = spec.to_owned();

        blocking(move || projects.resolve(&spec)).await
    }

    fn logs_in_as_root(&self, target: Option<&ResourceRef>) -> bool {
        let Some(target) = target else {
            return false;
        };

        match self.resolve(target.as_str()) {
            Ok(resolved) => resolved.logs_in_as_root(),
            Err(_) => true,
        }
    }
}

impl Resolved {
    pub(crate) fn logs_in_as_root(&self) -> bool {
        self.host().is_some_and(|host| is_root(host.user()))
    }
}

fn hidden_root(host: &ResourceRef) -> ActionError {
    ActionError::Failed(format!(
        "the ssh config for {host} changed to log in as root after the call was approved; \
         run it again so it is asked as privileged"
    ))
}

fn moved(host: &ResourceRef, reached: &str) -> ActionError {
    ActionError::Failed(format!(
        "{host} now leads to {reached}, which is not what the call was admitted for; \
         run it again so it is checked for the new place"
    ))
}
