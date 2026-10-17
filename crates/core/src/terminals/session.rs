mod attached;
mod ending;
mod kind;
mod origin;

pub use attached::AttachedSession;
pub use ending::Ending;
pub use kind::SessionKind;
pub(crate) use origin::Origin;
pub use synt_proto::SessionId;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bytes::Bytes;
use synt_actions::Context;
use synt_proto::EndReason;
use synt_ssh::{ShellEnd, TerminalSize};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::{Attachment, Block, Console, Screen};
use crate::CoreError;
use crate::clock::milliseconds;
use crate::link::Link;
use crate::locked::locked;

const CLOSE_GRACE: Duration = Duration::from_secs(5);

pub(crate) struct Session {
    id: SessionId,
    origin: Origin,
    console: Box<dyn Console>,
    input: mpsc::Sender<Bytes>,
    screen: AsyncMutex<Screen>,
    started: Instant,
    requested_end: Mutex<Option<EndReason>>,
    stop: CancellationToken,
    finished: CancellationToken,
    ending: Mutex<Option<Ending>>,
}

impl Session {
    pub(super) fn new(
        id: SessionId,
        origin: Origin,
        console: Box<dyn Console>,
        size: TerminalSize,
        input: mpsc::Sender<Bytes>,
    ) -> Arc<Self> {
        Arc::new(Self {
            id,
            origin,
            console,
            input,
            screen: AsyncMutex::new(Screen::new(id, size)),
            started: Instant::now(),
            requested_end: Mutex::new(None),
            stop: CancellationToken::new(),
            finished: CancellationToken::new(),
            ending: Mutex::new(None),
        })
    }

    pub(crate) fn id(&self) -> SessionId {
        self.id
    }

    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }

    pub(crate) fn host(&self) -> String {
        self.console.label()
    }

    pub(crate) fn kind(&self) -> SessionKind {
        self.console.kind()
    }

    pub(crate) fn age(&self) -> Duration {
        self.started.elapsed()
    }

    pub(crate) async fn size(&self) -> TerminalSize {
        self.screen.lock().await.size()
    }

    pub(crate) async fn attached(&self) -> usize {
        self.screen.lock().await.attached()
    }

    pub(crate) async fn blocks(&self) -> Vec<Block> {
        self.screen.lock().await.blocks()
    }

    pub(crate) async fn attach(&self, link: &Arc<Link>) -> AttachedSession {
        let stream = link.open_stream(self.input.clone());
        let attachment = Attachment::new(stream, link);

        let mut screen = self.screen.lock().await;
        let replayed = screen.attach(attachment).await;
        let size = screen.size();

        AttachedSession {
            session: self.id,
            stream,
            columns: size.columns,
            rows: size.rows,
            replayed_bytes: replayed,
        }
    }

    pub(crate) async fn resize(&self, size: TerminalSize) -> Result<(), CoreError> {
        let mut screen = self.screen.lock().await;

        self.console.resize(size).await?;
        screen.resize(size);

        Ok(())
    }

    pub(crate) async fn close(&self, reason: EndReason) -> Option<Ending> {
        self.request_end(reason);

        let hung_up = self.console.hang_up().await;
        let ended = hung_up && self.wait_for_end().await;

        if !ended {
            self.console.close().await;
            self.stop.cancel();
            self.finished.cancelled().await;
        }

        self.ending()
    }

    pub(super) async fn discard(&self) {
        self.console.close().await;
    }

    pub(crate) fn close_when_revoked(self: &Arc<Self>, context: &Context) {
        let Some(hold) = context.hold() else {
            return;
        };
        let stop = context.stop().clone();
        let session = Arc::clone(self);

        tokio::spawn(async move {
            tokio::select! {
                () = stop.cancelled() => {
                    session.close(EndReason::Closed).await;
                }
                () = session.finished() => {}
            }

            drop(hold);
        });
    }

    pub(crate) async fn finished(&self) {
        self.finished.cancelled().await;
    }

    pub(super) async fn type_in(self: Arc<Self>, mut typed: mpsc::Receiver<Bytes>) {
        loop {
            let data = tokio::select! {
                () = self.finished.cancelled() => return,
                data = typed.recv() => data,
            };

            let Some(data) = data else {
                return;
            };

            if self.console.write(data).await.is_err() {
                return;
            }
        }
    }

    pub(super) fn stopped(&self) -> &CancellationToken {
        &self.stop
    }

    pub(super) async fn show(&self, data: Bytes) {
        self.screen.lock().await.show(data).await;
    }

    pub(super) async fn end(&self, end: ShellEnd) -> Ending {
        let mut screen = self.screen.lock().await;
        let algorithms = self.console.algorithms();

        let ending = Ending {
            session: self.id,
            reason: self.reason(&end),
            exit: end.exit,
            signal: end.signal,
            duration_ms: milliseconds(self.started.elapsed()),
            output_bytes: screen.output_bytes(),
            blocks: screen.block_count(),
            kex: algorithms.map(|negotiated| negotiated.kex.clone()),
            cipher: algorithms.map(|negotiated| negotiated.cipher.clone()),
        };

        screen.end(ending.clone()).await;
        *locked(&self.ending) = Some(ending.clone());

        ending
    }

    pub(super) fn mark_finished(&self) {
        self.finished.cancel();
    }

    async fn wait_for_end(&self) -> bool {
        time::timeout(CLOSE_GRACE, self.finished.cancelled())
            .await
            .is_ok()
    }

    fn request_end(&self, reason: EndReason) {
        let mut requested = locked(&self.requested_end);

        requested.get_or_insert(reason);
    }

    fn reason(&self, end: &ShellEnd) -> EndReason {
        let requested = *locked(&self.requested_end);

        match requested {
            Some(reason) => reason,
            None if end.lost => EndReason::ConnectionLost,
            None => EndReason::Exited,
        }
    }

    fn ending(&self) -> Option<Ending> {
        locked(&self.ending).clone()
    }
}
