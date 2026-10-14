use std::sync::Arc;

use tokio::sync::broadcast;

use super::Request;
use super::waiter::Waiter;
use crate::event::{Appended, NewEvent};
use crate::journal::Journal;
use crate::seq::Seq;

pub(super) struct Batch {
    events: Vec<NewEvent>,
    waiters: Vec<Waiter>,
}

impl Batch {
    pub(super) fn assign(head: Seq, requests: Vec<Request>) -> Self {
        let mut events = Vec::with_capacity(requests.len());
        let mut waiters = Vec::with_capacity(requests.len());
        let mut next = head.next();

        for request in requests {
            let first = next;

            for (index, event) in request.events.into_iter().enumerate() {
                let joins_call = request.opens_call && index > 0;
                events.push(if joins_call { event.call(first) } else { event });
                next = next.next();
            }

            waiters.push(Waiter::new(request.reply, first));
        }

        Self { events, waiters }
    }

    pub(super) fn commit(self, journal: &mut Journal, watchers: &broadcast::Sender<Appended>) {
        let committed = journal.append_all(&self.events);

        if let Ok(seqs) = &committed {
            self.announce(watchers, seqs);
        }

        let failure = committed.err().map(Arc::new);

        for waiter in self.waiters {
            waiter.answer(failure.as_ref());
        }
    }

    fn announce(&self, watchers: &broadcast::Sender<Appended>, seqs: &[Seq]) {
        if watchers.receiver_count() == 0 {
            return;
        }

        for (seq, event) in seqs.iter().zip(&self.events) {
            let _ = watchers.send(Appended::new(*seq, event));
        }
    }
}
