use synt_actions::{ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Target;

use super::privilege::is_root;
use crate::blocking::blocking;
use crate::projects::{Projects, Resolved};

pub(crate) fn logs_in_as_root(projects: &Projects, target: Option<&ResourceRef>) -> bool {
    let Some(target) = target else {
        return false;
    };

    match projects.resolve(target.as_str()) {
        Ok(resolved) => is_root(resolved.host().user()),
        Err(_) => true,
    }
}

pub(crate) fn destination(projects: &Projects, target: Option<&ResourceRef>) -> Option<String> {
    let target = target?;
    let resolved = projects.resolve(target.as_str()).ok()?;

    Some(projects.destination(target.as_str(), &resolved))
}

pub(crate) async fn reach_host(
    projects: &Projects,
    context: &Context,
) -> Result<Target, ActionError> {
    let host = context.target().ok_or(ActionError::TargetRequired)?;
    let resolved = resolve_any(projects, host.as_str()).await?;

    let Resolved::Host(target) = &resolved else {
        return Err(ActionError::failed(format!(
            "{host} is a tunnel, not a host; open it with tunnel.open"
        )));
    };

    check_reach(projects, context, &resolved)?;

    Ok(target.clone())
}

pub(crate) fn check_reach(
    projects: &Projects,
    context: &Context,
    resolved: &Resolved,
) -> Result<(), ActionError> {
    let host = context.target().ok_or(ActionError::TargetRequired)?;

    if is_root(resolved.host().user()) && context.risk() != Risk::Privileged {
        return Err(hidden_root(host));
    }

    let reached = projects.destination(host.as_str(), resolved);

    if context.destination() != Some(reached.as_str()) {
        return Err(moved(host, &reached));
    }

    Ok(())
}

pub(crate) async fn resolve_any(projects: &Projects, spec: &str) -> Result<Resolved, ActionError> {
    let projects = projects.clone();
    let spec = spec.to_owned();

    blocking(move || projects.resolve(&spec))
        .await
        .map_err(|error| ActionError::Failed(error.to_string()))
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
