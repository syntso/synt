use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use synt_actions::Risk;
use tokio_util::sync::CancellationToken;

use super::{ForwardInfo, OpenForward};
use crate::locked::locked;

type Open = BTreeMap<u64, OpenForward>;

#[derive(Clone, Default)]
pub(crate) struct Forwards {
    open: Arc<Mutex<Open>>,
    issued: Arc<AtomicU64>,
}

impl Forwards {
    pub(crate) fn add(&self, forward: OpenForward) -> ForwardInfo {
        let id = self.issued.fetch_add(1, Ordering::Relaxed) + 1;
        let info = forward.info(id);
        let stop = forward.stop().clone();

        self.lock().insert(id, forward);
        self.close_when(id, stop);

        info
    }

    pub(crate) fn list(&self) -> Vec<ForwardInfo> {
        self.lock()
            .iter()
            .map(|(id, forward)| forward.info(*id))
            .collect()
    }

    pub(crate) fn risk(&self, id: u64) -> Option<Risk> {
        self.lock().get(&id).map(OpenForward::risk)
    }

    pub(crate) async fn close(&self, id: u64) -> Option<ForwardInfo> {
        let open = self.lock().remove(&id)?;

        Some(open.close(id).await)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn close_when(&self, id: u64, stop: CancellationToken) {
        let forwards = self.clone();

        tokio::spawn(async move {
            stop.cancelled().await;
            forwards.close(id).await;
        });
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        locked(&self.open)
    }
}

impl Debug for Forwards {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Forwards")
            .field("open", &self.lock().len())
            .finish_non_exhaustive()
    }
}
