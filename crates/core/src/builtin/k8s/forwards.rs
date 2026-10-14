mod close;
mod entry;
mod id;
mod info;
mod list;
mod open;

pub(crate) use close::K8sPortForwardClose;
use entry::ForwardEntry;
pub(crate) use id::ForwardId;
use info::ForwardInfo;
pub(crate) use list::K8sPortForwardList;
pub(crate) use open::K8sPortForward;

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use synt_actions::Risk;
use tokio_util::sync::CancellationToken;

use crate::locked::locked;

type Open = BTreeMap<ForwardId, ForwardEntry>;

#[derive(Clone, Default)]
pub(crate) struct Forwards {
    open: Arc<Mutex<Open>>,
    issued: Arc<AtomicU64>,
}

impl Forwards {
    pub(crate) fn add(&self, entry: ForwardEntry) -> ForwardInfo {
        let id = self.issue();
        let info = entry.info(id);
        let stop = entry.stop().clone();

        self.lock().insert(id, entry);
        self.close_when(id, stop);

        info
    }

    pub(crate) fn list(&self) -> Vec<ForwardInfo> {
        self.lock()
            .iter()
            .map(|(id, entry)| entry.info(*id))
            .collect()
    }

    pub(crate) fn risk(&self, id: ForwardId) -> Option<Risk> {
        self.lock().get(&id).map(ForwardEntry::risk)
    }

    pub(crate) async fn close(&self, id: ForwardId) -> Option<ForwardInfo> {
        let entry = self.lock().remove(&id)?;

        Some(entry.close(id).await)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    fn issue(&self) -> ForwardId {
        let number = self.issued.fetch_add(1, Ordering::Relaxed) + 1;

        ForwardId::new(number)
    }

    fn close_when(&self, id: ForwardId, stop: CancellationToken) {
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
