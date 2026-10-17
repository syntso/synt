use std::sync::Arc;

use tokio::sync::oneshot;

use crate::error::JournalError;
use crate::seq::Seq;

pub(super) struct Waiter {
    reply: oneshot::Sender<Result<Seq, JournalError>>,
    seq: Seq,
}

impl Waiter {
    pub(super) fn new(reply: oneshot::Sender<Result<Seq, JournalError>>, seq: Seq) -> Self {
        Self { reply, seq }
    }

    pub(super) fn answer(self, failure: Option<&Arc<JournalError>>) {
        let answer = match failure {
            None => Ok(self.seq),
            Some(error) => Err(JournalError::Commit(Arc::clone(error))),
        };

        let _ = self.reply.send(answer);
    }
}
