use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use tokio::sync::oneshot;

use crate::anchor::KeyStore;
use crate::chain::JournalId;
use crate::error::JournalError;
use crate::journal::Journal;
use crate::model::{NewEvent, Seq};
use crate::reader::JournalReader;

const MAX_BATCH: usize = 256;

#[derive(Clone)]
pub struct JournalWriter {
    inner: Arc<Inner>,
}

struct Inner {
    id: JournalId,
    path: PathBuf,
    keys: Arc<dyn KeyStore>,
    requests: Option<Sender<Request>>,
    thread: Option<JoinHandle<()>>,
}

struct Request {
    events: Vec<NewEvent>,
    opens_call: bool,
    reply: oneshot::Sender<Result<Seq, JournalError>>,
}

impl JournalWriter {
    pub fn spawn(journal: Journal) -> io::Result<Self> {
        let id = journal.id();
        let path = journal.path().to_owned();
        let keys = journal.keys();

        let (requests, incoming) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("synt-journal-{id}"))
            .spawn(move || serve(journal, &incoming))?;

        let inner = Inner {
            id,
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

fn serve(mut journal: Journal, incoming: &Receiver<Request>) {
    while let Ok(first) = incoming.recv() {
        let batch: Vec<Request> = std::iter::once(first)
            .chain(incoming.try_iter().take(MAX_BATCH - 1))
            .collect();

        let (events, waiting) = assign(journal.anchors().head.seq, batch);

        match journal.append_all(&events) {
            Ok(_) => {
                for (reply, seq) in waiting {
                    let _ = reply.send(Ok(seq));
                }
            }
            Err(error) => {
                let error = Arc::new(error);

                for (reply, _) in waiting {
                    let _ = reply.send(Err(JournalError::Commit(Arc::clone(&error))));
                }
            }
        }
    }
}

type Waiting = (oneshot::Sender<Result<Seq, JournalError>>, Seq);

fn assign(head: Seq, batch: Vec<Request>) -> (Vec<NewEvent>, Vec<Waiting>) {
    let mut events = Vec::with_capacity(batch.len());
    let mut waiting = Vec::with_capacity(batch.len());
    let mut next = head.next();

    for request in batch {
        let first = next;

        for (index, event) in request.events.into_iter().enumerate() {
            let event = if request.opens_call && index > 0 {
                event.call(first)
            } else {
                event
            };

            events.push(event);
            next = next.next();
        }

        waiting.push((request.reply, first));
    }

    (events, waiting)
}
