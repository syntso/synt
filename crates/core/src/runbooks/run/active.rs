use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub(super) struct ActiveRun {
    runbook: String,
    stop: CancellationToken,
}

impl ActiveRun {
    pub(super) fn new(runbook: &str, stop: CancellationToken) -> Self {
        Self {
            runbook: runbook.to_owned(),
            stop,
        }
    }

    pub(super) fn stop(self) -> String {
        self.stop.cancel();

        self.runbook
    }
}
