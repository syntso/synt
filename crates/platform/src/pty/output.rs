use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::{self, Instant};

use super::PtyEnd;

const DRAIN_AFTER_EXIT: Duration = Duration::from_millis(500);
const EXIT_AFTER_CLOSE: Duration = Duration::from_secs(2);

pub struct PtyOutput {
    data: mpsc::Receiver<Vec<u8>>,
    exit: oneshot::Receiver<PtyEnd>,
    end: PtyEnd,
    drain_until: Option<Instant>,
    finished: bool,
}

impl PtyOutput {
    pub(super) fn new(data: mpsc::Receiver<Vec<u8>>, exit: oneshot::Receiver<PtyEnd>) -> Self {
        Self {
            data,
            exit,
            end: PtyEnd::default(),
            drain_until: None,
            finished: false,
        }
    }

    pub async fn next(&mut self) -> Option<Vec<u8>> {
        while !self.finished {
            if let Some(data) = self.receive().await {
                return Some(data);
            }
        }

        None
    }

    pub fn end(&self) -> PtyEnd {
        self.end.clone()
    }

    async fn receive(&mut self) -> Option<Vec<u8>> {
        if let Some(deadline) = self.drain_until {
            return self.drain(deadline).await;
        }

        tokio::select! {
            biased;
            data = self.data.recv() => self.shown(data).await,
            end = &mut self.exit => {
                self.end = end.unwrap_or_default();
                self.drain_until = Some(Instant::now() + DRAIN_AFTER_EXIT);
                None
            }
        }
    }

    async fn shown(&mut self, data: Option<Vec<u8>>) -> Option<Vec<u8>> {
        if data.is_none() {
            self.close().await;
        }

        data
    }

    async fn drain(&mut self, deadline: Instant) -> Option<Vec<u8>> {
        let data = time::timeout_at(deadline, self.data.recv()).await;

        if let Ok(Some(data)) = data {
            return Some(data);
        }

        self.finished = true;
        None
    }

    async fn close(&mut self) {
        let end = time::timeout(EXIT_AFTER_CLOSE, &mut self.exit).await;

        if let Ok(Ok(end)) = end {
            self.end = end;
        }

        self.finished = true;
    }
}
