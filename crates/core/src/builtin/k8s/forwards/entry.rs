use std::sync::Arc;

use synt_actions::{Context, Risk};
use synt_kube::{Cluster, PortForward};
use tokio_util::sync::CancellationToken;

use super::ForwardInfo;

pub(crate) struct OpenForward {
    forward: PortForward,
    cluster: String,
    risk: Risk,
    stop: CancellationToken,
    _connection: Arc<Cluster>,
}

impl OpenForward {
    pub(crate) fn new(forward: PortForward, context: &Context, connection: Arc<Cluster>) -> Self {
        Self {
            forward,
            cluster: context.destination().unwrap_or_default().to_owned(),
            risk: context.risk(),
            stop: context.stop().clone(),
            _connection: connection,
        }
    }

    pub(crate) fn info(&self, id: u64) -> ForwardInfo {
        ForwardInfo {
            id,
            cluster: self.cluster.clone(),
            status: self.forward.status(),
        }
    }

    pub(crate) fn risk(&self) -> Risk {
        self.risk
    }

    pub(crate) fn stop(&self) -> &CancellationToken {
        &self.stop
    }

    pub(crate) async fn close(self, id: u64) -> ForwardInfo {
        self.stop.cancel();

        ForwardInfo {
            id,
            cluster: self.cluster,
            status: self.forward.close().await,
        }
    }
}
