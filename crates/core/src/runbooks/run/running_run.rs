use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub(crate) struct RunningRun {
    pub(crate) runbook: String,
    pub(crate) stop: CancellationToken,
}
