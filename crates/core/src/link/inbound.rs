use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bytes::Bytes;
use synt_actions::ActionError;
use synt_proto::{Credit, Frame, StreamId};
use tokio::sync::mpsc;
use tokio::time;

use super::{Delivery, Link, Route};

const WINDOW: u64 = 4 * 1024 * 1024;
const FRAMES_QUEUED: usize = 256;
const SILENCE_LIMIT: Duration = Duration::from_secs(60);

pub(crate) struct Inbound {
    link: Arc<Link>,
    stream: StreamId,
    data: mpsc::Receiver<Bytes>,
    allowance: Arc<AtomicU64>,
    received: u64,
    granted: u64,
}

impl Inbound {
    pub(crate) async fn open(link: &Arc<Link>, stream: StreamId) -> Result<Self, ActionError> {
        let (input, data) = mpsc::channel(FRAMES_QUEUED);
        let allowance = Arc::new(AtomicU64::new(0));

        let route = Route {
            input,
            delivery: Delivery::Metered {
                allowance: Arc::clone(&allowance),
                received: 0,
            },
        };

        if !link.claim(stream, route) {
            return Err(unusable(stream));
        }

        let mut inbound = Self {
            link: Arc::clone(link),
            stream,
            data,
            allowance,
            received: 0,
            granted: 0,
        };
        inbound.grant().await?;

        Ok(inbound)
    }

    pub(crate) async fn next(&mut self) -> Result<Option<Bytes>, ActionError> {
        let Ok(next) = time::timeout(SILENCE_LIMIT, self.data.recv()).await else {
            return Err(ActionError::failed(format!(
                "the client sent nothing for {} s",
                SILENCE_LIMIT.as_secs()
            )));
        };

        let Some(data) = next else {
            return Ok(None);
        };

        self.received += u64::try_from(data.len()).unwrap_or(u64::MAX);

        if self.received + WINDOW / 2 >= self.granted {
            self.grant().await?;
        }

        Ok(Some(data))
    }

    async fn grant(&mut self) -> Result<(), ActionError> {
        self.granted = self.received + WINDOW;
        self.allowance.store(self.granted, Ordering::Release);

        let credit = Frame::Stream {
            stream: self.stream,
            data: Credit::new(self.granted).encode(),
        };

        let sent = time::timeout(SILENCE_LIMIT, self.link.frames().send(credit)).await;

        match sent {
            Ok(Ok(())) => Ok(()),
            _ => Err(ActionError::failed("the client is gone")),
        }
    }
}

impl Drop for Inbound {
    fn drop(&mut self) {
        self.link.forget(self.stream);
    }
}

pub(super) fn unusable(stream: StreamId) -> ActionError {
    ActionError::InvalidParams(format!(
        "stream {} is not free, or was not opened by the client",
        stream.get()
    ))
}
