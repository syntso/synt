use synt_actions::ActionError;
use tokio::runtime::Handle;
use tokio::task;

use crate::extension::Answer;

pub(super) async fn on_guest_thread<T: Send + 'static>(
    work: impl FnOnce(Handle) -> Answer<T> + Send + 'static,
) -> Result<T, ActionError> {
    let runtime = Handle::current();

    let answer = task::spawn_blocking(move || work(runtime))
        .await
        .map_err(|_| ActionError::failed("the extension stopped unexpectedly"))?;

    match answer {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(error)) => Err(ActionError::failed(format!(
            "{}: {}",
            error.code, error.message
        ))),
        Err(error) => Err(ActionError::failed(error.to_string())),
    }
}
