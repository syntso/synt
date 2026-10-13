use tokio::task;

use crate::CoreError;

pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, CoreError> + Send + 'static,
) -> Result<T, CoreError> {
    task::spawn_blocking(work)
        .await
        .map_err(|_| CoreError::Stopped)?
}
