use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use synt_platform::Lock;
use tokio_util::sync::CancellationToken;

use super::RunGuard;
use super::active::ActiveRun;
use crate::runbooks::RunbookError;

type Active = HashMap<u64, ActiveRun>;

#[derive(Debug, Clone, Default)]
pub struct Runs {
    active: Arc<Mutex<Active>>,
}

impl Runs {
    pub(crate) fn start(&self, run: u64, runbook: &str, parent: &CancellationToken) -> RunGuard {
        let stop = parent.child_token();

        self.lock()
            .insert(run, ActiveRun::new(runbook, stop.clone()));

        RunGuard::new(run, self.clone(), stop)
    }

    pub(crate) fn stop(&self, run: u64) -> Result<String, RunbookError> {
        let active = self
            .lock()
            .get(&run)
            .cloned()
            .ok_or(RunbookError::NotRunning(run))?;

        Ok(active.stop())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    pub(super) fn finish(&self, run: u64) {
        self.lock().remove(&run);
    }

    fn lock(&self) -> MutexGuard<'_, Active> {
        self.active.locked()
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use super::Runs;

    #[test]
    fn a_run_is_known_until_its_guard_goes() {
        let runs = Runs::default();

        let guard = runs.start(7, "@lab/deploy", &CancellationToken::new());
        assert!(!runs.is_empty());

        drop(guard);
        assert!(runs.is_empty());
        assert!(runs.stop(7).is_err());
    }

    #[test]
    fn stopping_a_run_cancels_its_token() {
        let runs = Runs::default();
        let guard = runs.start(7, "@lab/deploy", &CancellationToken::new());

        assert_eq!(runs.stop(7).unwrap(), "@lab/deploy");
        assert!(guard.stop().is_cancelled());
    }

    #[test]
    fn a_run_stops_with_the_call_that_started_it() {
        let runs = Runs::default();
        let parent = CancellationToken::new();
        let guard = runs.start(7, "@lab/deploy", &parent);

        parent.cancel();

        assert!(guard.stop().is_cancelled());
    }
}
