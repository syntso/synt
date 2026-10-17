use tokio::sync::oneshot;

use crate::error::JournalError;
use crate::event::NewEvent;
use crate::seq::Seq;

pub(super) struct Request {
    pub(super) events: Vec<NewEvent>,
    pub(super) opens_call: bool,
    pub(super) reply: oneshot::Sender<Result<Seq, JournalError>>,
}
