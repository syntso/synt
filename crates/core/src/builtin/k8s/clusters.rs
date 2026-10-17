mod entry;
mod key;
mod reach;
mod ssh_dialer;

use entry::ClusterEntry;
use key::ClusterKey;
pub(crate) use reach::Reach;
use ssh_dialer::SshDialer;

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use synt_kube::{Cluster, Dialer};
use synt_platform::Lock;
use synt_ssh::{Pool, Target};
use tokio::time;

use super::K8sError;
use crate::builtin::ssh::within_connect_limit;

const IDLE_LIMIT: Duration = Duration::from_mins(1);
const LOOK_EVERY: Duration = Duration::from_secs(10);

type Open = HashMap<ClusterKey, ClusterEntry>;

#[derive(Clone)]
pub(crate) struct Clusters {
    pool: Pool,
    open: Arc<Mutex<Open>>,
}

impl Clusters {
    pub(crate) fn new(pool: Pool) -> Self {
        Self {
            pool,
            open: Arc::default(),
        }
    }

    pub(crate) fn cached(&self, reach: &Reach) -> Option<Arc<Cluster>> {
        self.lock().get_mut(&reach.key()).map(ClusterEntry::take)
    }

    pub(crate) async fn connect(&self, reach: &Reach) -> Result<Arc<Cluster>, K8sError> {
        if let Some(cluster) = self.cached(reach) {
            return Ok(cluster);
        }

        let dialer = match reach.via() {
            Some(via) => Some(self.dialer_through(via).await?),
            None => None,
        };

        let cluster = Arc::new(Cluster::connect(reach.spec(), dialer).await?);
        self.keep(reach.key(), Arc::clone(&cluster));

        Ok(cluster)
    }

    async fn dialer_through(&self, via: &Target) -> Result<Arc<dyn Dialer>, K8sError> {
        let lease = within_connect_limit(self.pool.acquire(via)).await?;

        Ok(Arc::new(SshDialer::new(lease)))
    }

    fn keep(&self, key: ClusterKey, cluster: Arc<Cluster>) {
        self.lock().insert(key.clone(), ClusterEntry::new(cluster));
        self.forget_when_idle(key);
    }

    fn forget_when_idle(&self, key: ClusterKey) {
        let open = Arc::clone(&self.open);

        tokio::spawn(async move {
            loop {
                time::sleep(LOOK_EVERY).await;

                let mut open = open.locked();
                let Some(entry) = open.get(&key) else {
                    return;
                };

                if entry.is_idle(IDLE_LIMIT) {
                    open.remove(&key);
                    return;
                }
            }
        });
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        self.open.locked()
    }
}

impl Debug for Clusters {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Clusters")
            .field("open", &self.lock().len())
            .finish_non_exhaustive()
    }
}
