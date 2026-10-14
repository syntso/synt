use synt_actions::ActionError;

use crate::CoreError;

pub(super) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, CoreError> + Send + 'static,
) -> Result<T, ActionError> {
    Ok(crate::blocking::blocking(work).await?)
}
