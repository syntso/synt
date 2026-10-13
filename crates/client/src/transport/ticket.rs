use std::sync::Arc;

use synt_proto::{Outcome, RequestId};
use tokio::sync::oneshot;

use super::pending::Pending;
use crate::ClientError;

pub(super) struct Ticket {
    id: RequestId,
    pending: Arc<Pending>,
    reply: oneshot::Receiver<Outcome>,
}

impl Ticket {
    pub(super) fn new(
        id: RequestId,
        pending: Arc<Pending>,
        reply: oneshot::Receiver<Outcome>,
    ) -> Self {
        Self { id, pending, reply }
    }

    pub(super) async fn reply(mut self) -> Result<Outcome, ClientError> {
        (&mut self.reply)
            .await
            .map_err(|_| ClientError::Disconnected)
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        self.pending.forget(self.id);
    }
}
