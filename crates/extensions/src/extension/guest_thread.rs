use synt_actions::ActionError;
use tokio::runtime::Handle;
use tokio::task;

use super::Answer;

pub(crate) async fn on_guest_thread<T: Send + 'static>(
    work: impl FnOnce(Handle) -> Answer<T> + Send + 'static,
) -> Result<T, ActionError> {
    let runtime = Handle::current();

    let answer = task::spawn_blocking(move || work(runtime))
        .await
        .map_err(|_| ActionError::failed("the extension stopped unexpectedly"))?;

    Ok(answer??)
}
