use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use bytes::Bytes;
use futures_util::future::join_all;
use synt_journal::{ActionId, EventKind, JournalWriter};
use synt_proto::EndReason;
use synt_ssh::TerminalSize;
use tokio::sync::mpsc;

use super::{Console, ConsoleOutput, Ending, Origin, SessionId, Terminal, pump};
use crate::recorder::{Call, Recorder};

type Open = HashMap<SessionId, Arc<Terminal>>;

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
        let open: Vec<Arc<Terminal>> = {
            let open = self.lock();
            self.closing.store(true, Ordering::SeqCst);
            open.values().map(Arc::clone).collect()
        };

        let closing = open
            .iter()
            .map(|terminal| terminal.close(EndReason::CoreStopped));

        join_all(closing).await;
    }

    pub(crate) fn start(
        &self,
        id: SessionId,
        origin: Origin,
        (console, output): (Console, ConsoleOutput),
        size: TerminalSize,
    ) -> Option<Arc<Terminal>> {
        let (terminal, typed) = self.admit(id, origin, console, size)?;

        tokio::spawn(Arc::clone(&terminal).type_in(typed));
        tokio::spawn(pump(self.clone(), Arc::clone(&terminal), output));

        Some(terminal)
    }

    pub(crate) fn get(&self, id: SessionId) -> Option<Arc<Terminal>> {
        self.lock().get(&id).map(Arc::clone)
    }

    pub(crate) fn all(&self) -> Vec<Arc<Terminal>> {
        let mut open: Vec<Arc<Terminal>> = self.lock().values().map(Arc::clone).collect();
        open.sort_by_key(|terminal| terminal.id());

        open
    }

    pub(super) async fn retire(&self, terminal: &Terminal, ending: &Ending) {
        self.lock().remove(&terminal.id());

        let origin = terminal.origin();

        let Ok(action) = ActionId::new(origin.action) else {
            return;
        };

        let call = Call {
            actor: &origin.actor,
            action: &action,
            target: origin.target.as_ref(),
        };

        let finished = Recorder::step(&call, EventKind::Finished, ending);
        let recorded = self
            .recorder
            .follow_up(origin.request, vec![finished])
            .await;

        if let Err(error) = recorded {
            eprintln!(
                "synt-core: journal: session {} ended but was not journaled: {error}",
                terminal.id()
            );
        }
    }

    fn admit(
        &self,
        id: SessionId,
        origin: Origin,
        console: Console,
        size: TerminalSize,
    ) -> Option<(Arc<Terminal>, mpsc::Receiver<Bytes>)> {
        let mut open = self.lock();

        if self.closing.load(Ordering::SeqCst) {
            drop(open);
            tokio::spawn(async move { console.close().await });
            return None;
        }

        let (terminal, typed) = Terminal::new(id, origin, console, size);
        open.insert(id, Arc::clone(&terminal));

        Some((terminal, typed))
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
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
