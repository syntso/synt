use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use synt_proto::{ApprovalDecision, ApprovalId};
use tokio::sync::oneshot;

use super::open_question::OpenQuestion;
use crate::locked::locked;

type Questions = HashMap<ApprovalId, oneshot::Sender<ApprovalDecision>>;

#[derive(Debug, Default)]
pub(super) struct Waiting {
    questions: Mutex<Questions>,
}

impl Waiting {
    pub(super) fn open(self: &Arc<Self>, id: ApprovalId) -> OpenQuestion {
        let (sender, answer) = oneshot::channel();
        self.lock().insert(id, sender);

        OpenQuestion::new(id, Arc::clone(self), answer)
    }

    pub(super) fn answer(&self, id: ApprovalId, decision: ApprovalDecision) -> bool {
        let Some(sender) = self.lock().remove(&id) else {
            return false;
        };

        sender.send(decision).is_ok()
    }

    pub(super) fn forget(&self, id: ApprovalId) {
        self.lock().remove(&id);
    }

    fn lock(&self) -> MutexGuard<'_, Questions> {
        locked(&self.questions)
    }
}
