use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use tokio::sync::{broadcast, oneshot};

use super::request::Request;
use super::worker::serve;
use crate::anchor::KeyStore;
use crate::chain::JournalId;
use crate::error::JournalError;
use crate::journal::Journal;
use crate::model::{Appended, NewEvent, Seq};
use crate::reader::JournalReader;

const WATCH_BACKLOG: usize = 1024;

#[derive(Clone)]
pub struct JournalWriter {
    inner: Arc<Inner>,
}

struct Inner {
    id: JournalId,
    watchers: broadcast::Sender<Appended>,
    path: PathBuf,
    keys: Arc<dyn KeyStore>,
    requests: Option<Sender<Request>>,
    thread: Option<JoinHandle<()>>,
}

impl JournalWriter {
    pub fn spawn(journal: Journal) -> io::Result<Self> {
        let id = journal.id();
        let path = journal.path().to_owned();
        let keys = journal.keys();

        let (requests, incoming) = mpsc::channel();
        let (watchers, _) = broadcast::channel(WATCH_BACKLOG);
        let announcing = watchers.clone();
        let thread = thread::Builder::new()
            .name(format!("synt-journal-{id}"))
            .spawn(move || serve(journal, &incoming, &announcing))?;

        let inner = Inner {
            id,
            watchers,
            path,
            keys,
            requests: Some(requests),
            thread: Some(thread),
        };
        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub fn id(&self) -> JournalId {
        self.inner.id
    }

    pub fn append(&self, event: NewEvent) -> Result<Seq, JournalError> {
        let answer = self.submit(vec![event], false)?;

        answer
            .blocking_recv()
            .map_err(|_| JournalError::WriterClosed)?
    }

    pub async fn append_async(&self, event: NewEvent) -> Result<Seq, JournalError> {
        self.append_together(vec![event]).await
    }

    pub async fn append_together(&self, events: Vec<NewEvent>) -> Result<Seq, JournalError> {
        let answer = self.submit(events, false)?;

        answer.await.map_err(|_| JournalError::WriterClosed)?
    }

    pub async fn open_call(
        &self,
        request: NewEvent,
        follow_ups: Vec<NewEvent>,
    ) -> Result<Seq, JournalError> {
        let events = std::iter::once(request).chain(follow_ups).collect();
        let answer = self.submit(events, true)?;

        answer.await.map_err(|_| JournalError::WriterClosed)?
    }

    pub fn watch(&self) -> broadcast::Receiver<Appended> {
        self.inner.watchers.subscribe()
    }

    pub fn reader(&self) -> Result<JournalReader, JournalError> {
        JournalReader::open(
            &self.inner.path,
            self.inner.id,
            Arc::clone(&self.inner.keys),
        )
    }

    fn submit(
        &self,
        events: Vec<NewEvent>,
        opens_call: bool,
    ) -> Result<oneshot::Receiver<Result<Seq, JournalError>>, JournalError> {
        if events.is_empty() {
            return Err(JournalError::NothingToAppend);
        }

        let requests = self
            .inner
            .requests
            .as_ref()
            .ok_or(JournalError::WriterClosed)?;
        let (reply, answer) = oneshot::channel();

        requests
            .send(Request {
                events,
                opens_call,
                reply,
            })
            .map_err(|_| JournalError::WriterClosed)?;

        Ok(answer)
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        drop(self.requests.take());

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
