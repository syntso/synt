use synt_actions::ActionError;
use tokio::runtime::Handle;
use tokio::task;

use crate::ExtensionError;

pub(crate) struct Blocking;

impl Blocking {
    pub(crate) async fn run<T: Send + 'static>(
        work: impl FnOnce() -> Result<T, ExtensionError> + Send + 'static,
    ) -> Result<T, ActionError> {
        let done = task::spawn_blocking(work)
            .await
            .map_err(|_| ActionError::failed("the work stopped unexpectedly"))?;

        Ok(done?)
    }

    pub(crate) async fn run_with_runtime<T: Send + 'static>(
        work: impl FnOnce(Handle) -> Result<T, ExtensionError> + Send + 'static,
    ) -> Result<T, ActionError> {
        let runtime = Handle::current();

        Self::run(move || work(runtime)).await
    }
}
