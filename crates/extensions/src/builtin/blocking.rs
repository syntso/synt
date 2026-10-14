use synt_actions::ActionError;
use tokio::task;

use crate::ExtensionError;

pub(super) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ExtensionError> + Send + 'static,
) -> Result<T, ActionError> {
    let done = task::spawn_blocking(work)
        .await
        .map_err(|_| ActionError::failed("the work stopped unexpectedly"))?;

    Ok(done?)
}
