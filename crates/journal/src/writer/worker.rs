use std::sync::Arc;
use std::sync::mpsc::Receiver;

use tokio::sync::oneshot;

use super::request::Request;
use crate::error::JournalError;
use crate::journal::Journal;
use crate::model::{NewEvent, Seq};

const MAX_BATCH: usize = 256;

pub(super) fn serve(mut journal: Journal, incoming: &Receiver<Request>) {
    while let Ok(first) = incoming.recv() {
        let batch: Vec<Request> = std::iter::once(first)
            .chain(incoming.try_iter().take(MAX_BATCH - 1))
            .collect();

        let (events, waiting) = assign(journal.anchors().head.seq, batch);
        let committed = journal.append_all(&events);

        answer(waiting, committed);
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

fn answer(waiting: Vec<Waiting>, committed: Result<Vec<Seq>, JournalError>) {
    let failure = committed.err().map(Arc::new);

    for (reply, seq) in waiting {
        let answer = match &failure {
            None => Ok(seq),
            Some(error) => Err(JournalError::Commit(Arc::clone(error))),
        };

        let _ = reply.send(answer);
    }
}
