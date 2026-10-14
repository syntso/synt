use tokio_util::sync::CancellationToken;

use super::Runs;

#[derive(Debug)]
pub(crate) struct RunGuard {
    run: u64,
    runs: Runs,
    stop: CancellationToken,
}

impl RunGuard {
    pub(super) fn new(run: u64, runs: Runs, stop: CancellationToken) -> Self {
        Self { run, runs, stop }
    }

    pub(crate) fn stop(&self) -> &CancellationToken {
        &self.stop
    }
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        self.runs.finish(self.run);
    }
}
