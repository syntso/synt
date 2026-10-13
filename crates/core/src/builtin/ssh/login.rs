use synt_actions::ActionError;
use synt_journal::ResourceRef;
use synt_ssh::Target;
use tokio::task;

use super::failure;
use super::privilege::is_root;

pub(crate) fn logs_in_as_root(target: Option<&ResourceRef>) -> bool {
    let Some(target) = target else {
        return false;
    };

    match Target::resolve(target.as_str()) {
        Ok(resolved) => is_root(resolved.user()),
        Err(_) => true,
    }
}

pub(crate) fn hidden_root(host: &ResourceRef) -> ActionError {
    ActionError::Failed(format!(
        "the ssh config for {host} changed to log in as root after the call was approved; \
         run it again so it is asked as privileged"
    ))
}

pub(crate) async fn resolve(spec: &str) -> Result<Target, ActionError> {
    let spec = spec.to_owned();

    let resolving = task::spawn_blocking(move || Target::resolve(&spec));

    let resolved = resolving
        .await
        .map_err(|_| ActionError::failed("reading the ssh config stopped unexpectedly"))?;

    resolved.map_err(|error| failure(&error))
}
