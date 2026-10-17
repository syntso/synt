use tokio::task::{self, JoinError};

pub async fn blocking<T, E, R>(work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Result<T, R>
where
    T: Send + 'static,
    E: From<JoinError> + Send + 'static,
    R: From<E>,
{
    let finished = task::spawn_blocking(work).await.map_err(E::from)?;

    Ok(finished?)
}
