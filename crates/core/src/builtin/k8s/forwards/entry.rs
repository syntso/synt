use std::sync::Arc;

use synt_actions::Risk;
use synt_kube::{Cluster, PortForward};
use tokio_util::sync::CancellationToken;

use super::ForwardInfo;

pub(crate) struct OpenForward {
    pub(crate) forward: PortForward,
    pub(crate) cluster: String,
    pub(crate) risk: Risk,
    pub(crate) stop: CancellationToken,
    pub(crate) _connection: Arc<Cluster>,
}

impl OpenForward {
    pub(crate) fn info(&self, id: u64) -> ForwardInfo {
        ForwardInfo {
            id,
            cluster: self.cluster.clone(),
            status: self.forward.status(),
        }
    }
}
