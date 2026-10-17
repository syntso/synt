use std::io;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use tokio::sync::broadcast;

use super::Request;
use super::batch::Batch;
use super::shared::Shared;
use crate::event::Appended;
use crate::journal::Journal;

const MAX_BATCH: usize = 256;
const WATCH_BACKLOG: usize = 1024;

pub(super) struct Worker {
    journal: Journal,
    incoming: Receiver<Request>,
    watchers: broadcast::Sender<Appended>,
}

impl Worker {
    pub(super) fn start(journal: Journal) -> io::Result<Shared> {
        let id = journal.id();
        let path = journal.path().to_owned();
        let keys = journal.keys();

        let (requests, incoming) = mpsc::channel();
        let (watchers, _) = broadcast::channel(WATCH_BACKLOG);

        let worker = Self {
            journal,
            incoming,
            watchers: watchers.clone(),
        };
        let thread = thread::Builder::new()
            .name(format!("synt-journal-{id}"))
            .spawn(move || worker.serve())?;

        Ok(Shared::new(id, path, keys, watchers, requests, thread))
    }

    fn serve(mut self) {
        while let Ok(first) = self.incoming.recv() {
            let requests: Vec<Request> = std::iter::once(first)
                .chain(self.incoming.try_iter().take(MAX_BATCH - 1))
                .collect();

            let head = self.journal.anchors().head.seq;
            let batch = Batch::assign(head, requests);

            batch.commit(&mut self.journal, &self.watchers);
        }
    }
}
