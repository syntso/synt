use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

use tokio::sync::broadcast;

use super::Request;
use crate::error::JournalError;
use crate::event::Appended;
use crate::journal::{JournalId, JournalReader};
use crate::key_store::KeyStore;

pub(super) struct Shared {
    id: JournalId,
    path: PathBuf,
    keys: Arc<dyn KeyStore>,
    watchers: broadcast::Sender<Appended>,
    requests: Option<Sender<Request>>,
    thread: Option<JoinHandle<()>>,
}

impl Shared {
    pub(super) fn new(
        id: JournalId,
        path: PathBuf,
        keys: Arc<dyn KeyStore>,
        watchers: broadcast::Sender<Appended>,
        requests: Sender<Request>,
        thread: JoinHandle<()>,
    ) -> Self {
        Self {
            id,
            path,
            keys,
            watchers,
            requests: Some(requests),
            thread: Some(thread),
        }
    }

    pub(super) fn id(&self) -> JournalId {
        self.id
    }

    pub(super) fn watch(&self) -> broadcast::Receiver<Appended> {
        self.watchers.subscribe()
    }

    pub(super) fn reader(&self) -> Result<JournalReader, JournalError> {
        JournalReader::open(&self.path, self.id, Arc::clone(&self.keys))
    }

    pub(super) fn send(&self, request: Request) -> Result<(), JournalError> {
        let requests = self.requests.as_ref().ok_or(JournalError::WriterClosed)?;
        requests.send(request)?;

        Ok(())
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        drop(self.requests.take());

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
