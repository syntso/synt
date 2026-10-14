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

impl Lane {
    pub(crate) fn start(guest: Arc<Guest>, handle: &Handle) -> Self {
        let (wakes, incoming) = mpsc::channel(QUEUED);
        let stop = CancellationToken::new();

        handle.spawn(drive(guest, incoming, stop.clone(), handle.clone()));

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

async fn drive(
    guest: Arc<Guest>,
    mut incoming: mpsc::Receiver<Wake>,
    stop: CancellationToken,
    handle: Handle,
) {
    loop {
        let wakes = tokio::select! {
            () = stop.cancelled() => return,
            next = incoming.recv() => match next {
                Some(wake) => vec![wake],
                None => return,
            },
            due = guest.timers().due() => due.into_iter().map(Wake::Timer).collect(),
        };

        for wake in wakes {
            perform(&guest, wake, &stop, &handle).await;
        }
    }
}

async fn perform(guest: &Arc<Guest>, wake: Wake, stop: &CancellationToken, handle: &Handle) {
    let what = wake.describe();
    let guest = Arc::clone(guest);
    let id = guest.manifest().id.clone();
    let stop = stop.clone();
    let handle = handle.clone();

    let outcome = task::spawn_blocking(move || guest.wake(&wake, stop, handle)).await;

    let problem = match outcome {
        Ok(Ok(Ok(()))) => return,
        Ok(Ok(Err(error))) => format!("{}: {}", error.code, error.message),
        Ok(Err(error)) => error.to_string(),
        Err(_) => "stopped unexpectedly".to_owned(),
    };

    eprintln!("synt-core: extension {id}: {what}: {problem}");
}
