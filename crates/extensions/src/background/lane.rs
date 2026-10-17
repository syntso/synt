mod driver;

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use driver::Driver;

use super::Wake;
use crate::extensions::Guest;

const QUEUED: usize = 64;

pub(crate) struct Lane {
    wakes: mpsc::Sender<Wake>,
    stop: CancellationToken,
}

impl Lane {
    pub(crate) fn start(guest: Arc<Guest>, background: &Handle) -> Self {
        let (wakes, incoming) = mpsc::channel(QUEUED);
        let stop = CancellationToken::new();

        let driver = Driver::new(guest, stop.clone(), background.clone());
        background.spawn(driver.drive(incoming));

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
