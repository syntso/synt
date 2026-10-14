use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use synt_actions::ActionError;
use synt_kube::{Cluster, Dialer};
use synt_ssh::Pool;
use tokio::time;

use super::{CachedCluster, Reach, SshDialer};
use crate::builtin::k8s::failure::failed;
use crate::builtin::ssh::within_connect_limit;
use crate::locked::locked;

const IDLE_LIMIT: Duration = Duration::from_mins(1);
const LOOK_EVERY: Duration = Duration::from_secs(10);

type Open = HashMap<String, CachedCluster>;

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
        self.lock().get_mut(&reach.key()).map(CachedCluster::take)
    }

    pub(crate) async fn connect(&self, reach: &Reach) -> Result<Arc<Cluster>, ActionError> {
        if let Some(cluster) = self.cached(reach) {
            return Ok(cluster);
        }

        let dialer = match &reach.via {
            Some(via) => {
                let lease = within_connect_limit(self.pool.acquire(via)).await?;
                Some(Arc::new(SshDialer::new(lease)) as Arc<dyn Dialer>)
            }
            None => None,
        };

        let cluster = Cluster::connect(&reach.spec, dialer)
            .await
            .map_err(failed)?;
        let cluster = Arc::new(cluster);

        self.keep(reach.key(), Arc::clone(&cluster));

        Ok(cluster)
    }

    fn keep(&self, key: String, cluster: Arc<Cluster>) {
        self.lock().insert(key.clone(), CachedCluster::new(cluster));

        let open = Arc::clone(&self.open);
        tokio::spawn(async move {
            loop {
                time::sleep(LOOK_EVERY).await;

                let mut open = locked(&open);
                let Some(cached) = open.get(&key) else {
                    return;
                };

                if cached.is_idle(IDLE_LIMIT) {
                    open.remove(&key);
                    return;
                }
            }
        });
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        locked(&self.open)
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
