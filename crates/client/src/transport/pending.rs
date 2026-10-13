use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use synt_proto::{Outcome, RequestId, Response};
use tokio::sync::oneshot;

use super::ticket::Ticket;
use crate::ClientError;

type Waiters = HashMap<RequestId, oneshot::Sender<Outcome>>;

pub(super) struct Pending {
    waiters: Mutex<Option<Waiters>>,
}

impl Pending {
    pub(super) fn new() -> Self {
        Self {
            waiters: Mutex::new(Some(Waiters::new())),
        }
    }

    pub(super) fn register(self: &Arc<Self>, id: RequestId) -> Result<Ticket, ClientError> {
        let (sender, reply) = oneshot::channel();

        let mut waiters = self.lock();
        let Some(open) = waiters.as_mut() else {
            return Err(ClientError::Disconnected);
        };
        open.insert(id, sender);
        drop(waiters);

        Ok(Ticket::new(id, Arc::clone(self), reply))
    }

    pub(super) fn resolve(&self, response: Response) {
        let Some(id) = response.id else {
            return;
        };

        let waiter = self.lock().as_mut().and_then(|open| open.remove(&id));

        if let Some(waiter) = waiter {
            let _ = waiter.send(response.outcome);
        }
    }

    pub(super) fn forget(&self, id: RequestId) {
        if let Some(open) = self.lock().as_mut() {
            open.remove(&id);
        }
    }

    pub(super) fn close(&self) {
        let abandoned = self.lock().take();
        drop(abandoned);
    }

    fn lock(&self) -> MutexGuard<'_, Option<Waiters>> {
        self.waiters.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use serde_json::json;
    use synt_proto::{Outcome, RequestId, Response};

    use super::Pending;
    use crate::ClientError;

    fn waiting(pending: &Pending) -> usize {
        pending.lock().as_ref().map_or(0, HashMap::len)
    }

    #[tokio::test]
    async fn a_response_reaches_its_ticket() {
        let pending = Arc::new(Pending::new());
        let ticket = pending.register(RequestId(3)).unwrap();

        pending.resolve(Response::new(Some(RequestId(3)), Ok(json!("pong"))));

        assert_eq!(
            ticket.reply().await.unwrap(),
            Outcome::Result(json!("pong"))
        );
        assert_eq!(waiting(&pending), 0);
    }

    #[test]
    fn responses_nobody_waits_for_are_ignored() {
        let pending = Arc::new(Pending::new());
        let _ticket = pending.register(RequestId(1)).unwrap();

        pending.resolve(Response::new(Some(RequestId(2)), Ok(json!(null))));
        pending.resolve(Response::new(None, Ok(json!(null))));

        assert_eq!(waiting(&pending), 1);
    }

    #[test]
    fn a_dropped_ticket_stops_waiting() {
        let pending = Arc::new(Pending::new());
        let ticket = pending.register(RequestId(1)).unwrap();

        drop(ticket);

        assert_eq!(waiting(&pending), 0);
    }

    #[tokio::test]
    async fn closing_wakes_every_ticket_with_disconnected() {
        let pending = Arc::new(Pending::new());
        let first = pending.register(RequestId(1)).unwrap();
        let second = pending.register(RequestId(2)).unwrap();

        pending.close();

        assert!(matches!(
            first.reply().await,
            Err(ClientError::Disconnected)
        ));
        assert!(matches!(
            second.reply().await,
            Err(ClientError::Disconnected)
        ));
    }

    #[test]
    fn nothing_registers_after_close() {
        let pending = Arc::new(Pending::new());
        pending.close();

        assert!(matches!(
            pending.register(RequestId(1)),
            Err(ClientError::Disconnected)
        ));
    }
}
