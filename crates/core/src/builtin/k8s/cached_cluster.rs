use std::sync::Arc;
use std::time::Duration;

use synt_kube::Cluster;
use tokio::time::Instant;

pub(crate) struct CachedCluster {
    pub(crate) cluster: Arc<Cluster>,
    pub(crate) used: Instant,
}

impl CachedCluster {
    pub(crate) fn new(cluster: Arc<Cluster>) -> Self {
        Self {
            cluster,
            used: Instant::now(),
        }
    }

    pub(crate) fn take(&mut self) -> Arc<Cluster> {
        self.used = Instant::now();

        Arc::clone(&self.cluster)
    }

    pub(crate) fn is_idle(&self, limit: Duration) -> bool {
        self.used.elapsed() >= limit && Arc::strong_count(&self.cluster) == 1
    }
}
