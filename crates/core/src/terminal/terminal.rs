use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use bytes::Bytes;
use synt_proto::EndReason;
use synt_ssh::{Shell, ShellEnd, TerminalSize};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::{Attached, Attachment, Ending, Origin, Screen, SessionId};
use crate::link::Link;

const INPUT_BACKLOG: usize = 256;
const CLOSE_GRACE: Duration = Duration::from_secs(5);

pub(crate) struct Terminal {
    id: SessionId,
    origin: Origin,
    shell: Shell,
    input: mpsc::Sender<Bytes>,
    screen: AsyncMutex<Screen>,
    started: Instant,
    requested_end: Mutex<Option<EndReason>>,
    stop: CancellationToken,
    finished: CancellationToken,
    ending: Mutex<Option<Ending>>,
}

impl Terminal {
    pub(super) fn new(
        id: SessionId,
        origin: Origin,
        shell: Shell,
        size: TerminalSize,
    ) -> (Arc<Self>, mpsc::Receiver<Bytes>) {
        let (input, typed) = mpsc::channel(INPUT_BACKLOG);

        let terminal = Arc::new(Self {
            id,
            origin,
            shell,
            input,
            screen: AsyncMutex::new(Screen::new(size)),
            started: Instant::now(),
            requested_end: Mutex::new(None),
            stop: CancellationToken::new(),
            finished: CancellationToken::new(),
            ending: Mutex::new(None),
        });

        (terminal, typed)
    }

    pub(crate) fn id(&self) -> SessionId {
        self.id
    }

    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }

    pub(crate) fn host(&self) -> String {
        self.shell.target().label()
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

    pub(crate) async fn attach(&self, link: &Arc<Link>) -> Attached {
        let stream = link.open_stream(self.input.clone());
        let attachment = Attachment::new(stream, link);

        let mut screen = self.screen.lock().await;
        let replayed = screen.attach(attachment).await;

        Attached {
            stream,
            size: screen.size(),
            replayed,
        }
    }

    pub(crate) async fn resize(&self, size: TerminalSize) -> Result<(), String> {
        let mut screen = self.screen.lock().await;

        self.shell
            .resize(size)
            .await
            .map_err(|error| error.to_string())?;
        screen.resize(size);

        Ok(())
    }

    pub(crate) async fn close(&self, reason: EndReason) -> Option<Ending> {
        self.request_end(reason);

        let hung_up = self.shell.hang_up().await.is_ok();
        let ended = hung_up && self.wait_for_end().await;

        if !ended {
            self.shell.close().await;
            self.stop.cancel();
            self.finished.cancelled().await;
        }

        self.ending()
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

            if self.shell.write(data).await.is_err() {
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

        let ending = Ending {
            session: self.id,
            reason: self.reason(&end),
            exit: end.exit,
            signal: end.signal,
            duration_ms: milliseconds(self.started.elapsed()),
            output_bytes: screen.output_bytes(),
        };

        screen.end(ending.clone()).await;
        *self.ending.lock().unwrap_or_else(PoisonError::into_inner) = Some(ending.clone());

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
        let mut requested = self
            .requested_end
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        requested.get_or_insert(reason);
    }

    fn reason(&self, end: &ShellEnd) -> EndReason {
        let requested = *self
            .requested_end
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        match requested {
            Some(reason) => reason,
            None if end.lost => EndReason::ConnectionLost,
            None => EndReason::Exited,
        }
    }

    fn ending(&self) -> Option<Ending> {
        self.ending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

fn milliseconds(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}
