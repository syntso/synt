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
    event: NewEvent,
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
        let answer = self.submit(event)?;

        answer
            .blocking_recv()
            .map_err(|_| JournalError::WriterClosed)?
    }

    pub async fn append_async(&self, event: NewEvent) -> Result<Seq, JournalError> {
        let answer = self.submit(event)?;

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
        event: NewEvent,
    ) -> Result<oneshot::Receiver<Result<Seq, JournalError>>, JournalError> {
        let requests = self
            .inner
            .requests
            .as_ref()
            .ok_or(JournalError::WriterClosed)?;
        let (reply, answer) = oneshot::channel();

        requests
            .send(Request { event, reply })
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

        let (events, replies): (Vec<NewEvent>, Vec<_>) = batch
            .into_iter()
            .map(|request| (request.event, request.reply))
            .unzip();

        match journal.append_all(&events) {
            Ok(seqs) => {
                for (reply, seq) in replies.into_iter().zip(seqs) {
                    let _ = reply.send(Ok(seq));
                }
            }
            Err(error) => {
                let error = Arc::new(error);

                for reply in replies {
                    let _ = reply.send(Err(JournalError::Commit(Arc::clone(&error))));
                }
            }
        }
    }
}
