mod batch;
mod request;
mod shared;
mod waiter;
mod worker;

use std::io;
use std::sync::Arc;

use tokio::sync::{broadcast, oneshot};

use self::request::Request;
use self::shared::Shared;
use self::worker::Worker;
use super::{Journal, JournalId};
use crate::error::JournalError;
use crate::event::{Appended, NewEvent};
use crate::journal::JournalReader;
use crate::seq::Seq;

#[derive(Clone)]
pub struct JournalWriter {
    shared: Arc<Shared>,
}

impl JournalWriter {
    pub fn spawn(journal: Journal) -> io::Result<Self> {
        let shared = Worker::start(journal)?;

        Ok(Self {
            shared: Arc::new(shared),
        })
    }

    pub fn id(&self) -> JournalId {
        self.shared.id()
    }

    pub fn append(&self, event: NewEvent) -> Result<Seq, JournalError> {
        let answer = self.submit(vec![event], false)?;

        answer.blocking_recv()?
    }

    pub async fn append_async(&self, event: NewEvent) -> Result<Seq, JournalError> {
        self.append_together(vec![event]).await
    }

    pub async fn append_together(&self, events: Vec<NewEvent>) -> Result<Seq, JournalError> {
        let answer = self.submit(events, false)?;

        answer.await?
    }

    pub async fn open_call(
        &self,
        request: NewEvent,
        follow_ups: Vec<NewEvent>,
    ) -> Result<Seq, JournalError> {
        let events = std::iter::once(request).chain(follow_ups).collect();
        let answer = self.submit(events, true)?;

        answer.await?
    }

    pub fn watch(&self) -> broadcast::Receiver<Appended> {
        self.shared.watch()
    }

    pub fn reader(&self) -> Result<JournalReader, JournalError> {
        self.shared.reader()
    }

    fn submit(
        &self,
        events: Vec<NewEvent>,
        opens_call: bool,
    ) -> Result<oneshot::Receiver<Result<Seq, JournalError>>, JournalError> {
        if events.is_empty() {
            return Err(JournalError::NothingToAppend);
        }

        let (reply, answer) = oneshot::channel();
        let request = Request {
            events,
            opens_call,
            reply,
        };

        self.shared.send(request)?;

        Ok(answer)
    }
}
