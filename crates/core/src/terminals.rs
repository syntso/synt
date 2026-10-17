mod block;
mod console;
mod screen;
mod session;

pub(crate) use block::{Block, BlockHistory, Scanner};
pub(crate) use console::{Console, ConsoleOutput, OpenConsole};
pub(crate) use session::Origin;
pub(crate) use session::Session;
pub use session::{AttachedSession, Ending, SessionId, SessionKind};

use screen::{Attachment, Screen};

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use futures_util::future::join_all;
use synt_journal::{ActionId, EventKind, JournalWriter};
use synt_proto::EndReason;
use synt_ssh::TerminalSize;
use tokio::sync::mpsc;

use crate::CoreError;
use crate::locked::locked;
use crate::log::log;
use crate::recorder::{Call, Recorder};

type Open = HashMap<SessionId, Arc<Session>>;

const INPUT_BACKLOG: usize = 256;

#[derive(Clone)]
pub struct Terminals {
    open: Arc<Mutex<Open>>,
    closing: Arc<AtomicBool>,
    recorder: Arc<Recorder>,
}

impl Terminals {
    pub fn new(journal: JournalWriter) -> Self {
        Self {
            open: Arc::new(Mutex::new(Open::new())),
            closing: Arc::new(AtomicBool::new(false)),
            recorder: Arc::new(Recorder::new(journal)),
        }
    }

    pub async fn close_all(&self) {
        let open: Vec<Arc<Session>> = {
            let open = self.lock();
            self.closing.store(true, Ordering::SeqCst);
            open.values().map(Arc::clone).collect()
        };

        let closing = open
            .iter()
            .map(|session| session.close(EndReason::CoreStopped));

        join_all(closing).await;
    }

    pub(crate) fn start(
        &self,
        id: SessionId,
        origin: Origin,
        opened: OpenConsole,
        size: TerminalSize,
    ) -> Option<Arc<Session>> {
        let OpenConsole { console, output } = opened;
        let (input, typed) = mpsc::channel(INPUT_BACKLOG);

        let session = self.admit(Session::new(id, origin, console, size, input))?;

        tokio::spawn(Arc::clone(&session).type_in(typed));
        tokio::spawn(self.clone().pump(Arc::clone(&session), output));

        Some(session)
    }

    pub(crate) fn get(&self, id: SessionId) -> Option<Arc<Session>> {
        self.lock().get(&id).map(Arc::clone)
    }

    pub(crate) fn find(&self, id: SessionId) -> Result<Arc<Session>, CoreError> {
        self.get(id).ok_or(CoreError::NoSession(id))
    }

    pub(crate) fn all(&self) -> Vec<Arc<Session>> {
        let mut open: Vec<Arc<Session>> = self.lock().values().map(Arc::clone).collect();
        open.sort_by_key(|session| session.id());

        open
    }

    fn admit(&self, session: Arc<Session>) -> Option<Arc<Session>> {
        let mut open = self.lock();

        if self.closing.load(Ordering::SeqCst) {
            drop(open);
            tokio::spawn(async move { session.discard().await });
            return None;
        }

        open.insert(session.id(), Arc::clone(&session));

        Some(session)
    }

    async fn pump(self, session: Arc<Session>, mut output: Box<dyn ConsoleOutput>) {
        loop {
            let data = tokio::select! {
                biased;
                () = session.stopped().cancelled() => break,
                data = output.next() => data,
            };

            let Some(data) = data else {
                break;
            };

            session.show(data).await;
        }

        let ending = session.end(output.end()).await;
        self.retire(&session, &ending).await;

        session.mark_finished();
    }

    async fn retire(&self, session: &Session, ending: &Ending) {
        self.lock().remove(&session.id());

        let origin = session.origin();

        let Ok(action) = ActionId::new(origin.action) else {
            return;
        };

        let call = Call {
            actor: &origin.actor,
            action: &action,
            target: origin.target.as_ref(),
        };

        let finished = call.step(EventKind::Finished, ending);
        let recorded = self
            .recorder
            .follow_up(origin.request, vec![finished])
            .await;

        if let Err(error) = recorded {
            log!(
                "journal: session {} ended but was not journaled: {error}",
                session.id()
            );
        }
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        locked(&self.open)
    }
}

impl Debug for Terminals {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Terminals")
            .field("open", &self.lock().len())
            .finish()
    }
}
