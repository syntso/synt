use std::sync::Arc;

use synt_proto::{ApprovalDecision, ApprovalId};
use tokio::sync::oneshot;

use super::Waiting;

pub(super) struct OpenQuestion {
    id: ApprovalId,
    waiting: Arc<Waiting>,
    answer: oneshot::Receiver<ApprovalDecision>,
}

impl OpenQuestion {
    pub(super) fn new(
        id: ApprovalId,
        waiting: Arc<Waiting>,
        answer: oneshot::Receiver<ApprovalDecision>,
    ) -> Self {
        Self {
            id,
            waiting,
            answer,
        }
    }

    pub(super) fn id(&self) -> ApprovalId {
        self.id
    }

    pub(super) async fn answer(&mut self) -> Option<ApprovalDecision> {
        (&mut self.answer).await.ok()
    }
}

impl Drop for OpenQuestion {
    fn drop(&mut self) {
        self.waiting.forget(self.id);
    }
}
