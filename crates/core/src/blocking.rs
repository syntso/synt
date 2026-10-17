use tokio::task;

use crate::CoreError;

pub(crate) async fn blocking<T, E>(
    work: impl FnOnce() -> Result<T, CoreError> + Send + 'static,
) -> Result<T, E>
where
    T: Send + 'static,
    E: From<CoreError>,
{
    let finished = task::spawn_blocking(work)
        .await
        .map_err(|_| CoreError::Stopped)?;

    Ok(finished?)
}
