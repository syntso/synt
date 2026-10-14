use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task;
use tokio_util::sync::CancellationToken;

use super::Wake;
use crate::extension::Guest;

const QUEUED: usize = 64;

pub(crate) struct Lane {
    wakes: mpsc::Sender<Wake>,
    stop: CancellationToken,
}

#[derive(Clone)]
struct Driver {
    guest: Arc<Guest>,
    stop: CancellationToken,
    handle: Handle,
}

impl Lane {
    pub(crate) fn start(guest: Arc<Guest>, handle: &Handle) -> Self {
        let (wakes, incoming) = mpsc::channel(QUEUED);
        let stop = CancellationToken::new();

        let driver = Driver {
            guest,
            stop: stop.clone(),
            handle: handle.clone(),
        };
        handle.spawn(driver.drive(incoming));

        Self { wakes, stop }
    }

    pub(crate) fn wake(&self, wake: Wake) -> bool {
        self.wakes.try_send(wake).is_ok()
    }
}

impl Drop for Lane {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

impl Driver {
    async fn drive(self, mut incoming: mpsc::Receiver<Wake>) {
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
        let id = self.guest.manifest().id.clone();
        let driver = self.clone();

        let outcome = task::spawn_blocking(move || {
            driver
                .guest
                .wake(&wake, driver.stop.clone(), driver.handle.clone())
        })
        .await;

        let problem = match outcome {
            Ok(Ok(Ok(()))) => return,
            Ok(Ok(Err(error))) => error.to_string(),
            Ok(Err(error)) => error.to_string(),
            Err(_) => "stopped unexpectedly".to_owned(),
        };

        eprintln!("synt-core: extension {id}: {what}: {problem}");
    }
}
