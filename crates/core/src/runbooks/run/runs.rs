use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio_util::sync::CancellationToken;

use super::{RunGuard, RunningRun};
use crate::locked::locked;

type Running = HashMap<u64, RunningRun>;

#[derive(Debug, Clone, Default)]
pub struct Runs {
    running: Arc<Mutex<Running>>,
}

impl Runs {
    pub(crate) fn start(&self, run: u64, runbook: &str, parent: &CancellationToken) -> RunGuard {
        let stop = parent.child_token();
        let running = RunningRun {
            runbook: runbook.to_owned(),
            stop: stop.clone(),
        };

        self.lock().insert(run, running);

        RunGuard::new(run, self.clone(), stop)
    }

    pub(crate) fn stop(&self, run: u64) -> Option<String> {
        let running = self.lock().get(&run).cloned()?;
        running.stop.cancel();

        Some(running.runbook)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    pub(super) fn finish(&self, run: u64) {
        self.lock().remove(&run);
    }

    fn lock(&self) -> MutexGuard<'_, Running> {
        locked(&self.running)
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
        assert_eq!(runs.stop(7), None);
    }

    #[test]
    fn stopping_a_run_cancels_its_token() {
        let runs = Runs::default();
        let guard = runs.start(7, "@lab/deploy", &CancellationToken::new());

        assert_eq!(runs.stop(7).as_deref(), Some("@lab/deploy"));
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
