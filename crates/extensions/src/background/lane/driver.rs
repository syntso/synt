use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task;
use tokio_util::sync::CancellationToken;

use crate::ExtensionError;
use crate::background::Wake;
use crate::extension::Guest;

#[derive(Clone)]
pub(super) struct Driver {
    guest: Arc<Guest>,
    stop: CancellationToken,
    runtime: Handle,
}

impl Driver {
    pub(super) fn new(guest: Arc<Guest>, stop: CancellationToken, runtime: Handle) -> Self {
        Self {
            guest,
            stop,
            runtime,
        }
    }

    pub(super) async fn drive(self, mut incoming: mpsc::Receiver<Wake>) {
        while let Some(wakes) = self.next(&mut incoming).await {
            for wake in wakes {
                self.perform(wake).await;
            }
        }
    }

    async fn next(&self, incoming: &mut mpsc::Receiver<Wake>) -> Option<Vec<Wake>> {
        tokio::select! {
            () = self.stop.cancelled() => None,
            next = incoming.recv() => next.map(|wake| vec![wake]),
            due = self.guest.timers().due() => Some(due.into_iter().map(Wake::Timer).collect()),
        }
    }

    async fn perform(&self, wake: Wake) {
        let what = wake.describe();
        let driver = self.clone();

        let outcome = task::spawn_blocking(move || driver.wake(&wake)).await;

        let problem = match outcome {
            Ok(Ok(())) => return,
            Ok(Err(error)) => error.to_string(),
            Err(_) => "stopped unexpectedly".to_owned(),
        };

        eprintln!(
            "synt-core: extension {}: {what}: {problem}",
            self.guest.id()
        );
    }

    fn wake(&self, wake: &Wake) -> Result<(), ExtensionError> {
        self.guest
            .wake(wake, self.stop.clone(), self.runtime.clone())
    }
}
