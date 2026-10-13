use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use synt_actions::ActionError;
use synt_proto::{Credit, Frame, StreamId};
use tokio::sync::mpsc;
use tokio::time;

use super::inbound::unusable;
use super::{Delivery, Link, Route};

const CREDITS_QUEUED: usize = 64;
const SILENCE_LIMIT: Duration = Duration::from_secs(60);

pub(crate) struct Outbound {
    link: Arc<Link>,
    stream: StreamId,
    frames: mpsc::Sender<Frame>,
    credits: mpsc::Receiver<Bytes>,
    allowed: u64,
    sent: u64,
}

impl Outbound {
    pub(crate) fn open(link: &Arc<Link>, stream: StreamId) -> Result<Self, ActionError> {
        let (input, credits) = mpsc::channel(CREDITS_QUEUED);

        let route = Route {
            input,
            delivery: Delivery::Strict,
        };

        if !link.claim(stream, route) {
            return Err(unusable(stream));
        }

        Ok(Self {
            link: Arc::clone(link),
            stream,
            frames: link.frames(),
            credits,
            allowed: Credit::OPENING.0,
            sent: 0,
        })
    }

    pub(crate) async fn send(&mut self, data: Bytes) -> Result<(), ActionError> {
        while self.sent >= self.allowed {
            self.await_credit().await?;
        }

        let length = u64::try_from(data.len()).unwrap_or(u64::MAX);
        self.forward(data).await?;
        self.sent += length;

        Ok(())
    }

    pub(crate) async fn finish(&mut self) {
        let _ = self.forward(Bytes::new()).await;
    }

    async fn await_credit(&mut self) -> Result<(), ActionError> {
        let Ok(next) = time::timeout(SILENCE_LIMIT, self.credits.recv()).await else {
            return Err(ActionError::failed(format!(
                "the client took nothing for {} s",
                SILENCE_LIMIT.as_secs()
            )));
        };

        let Some(data) = next else {
            return Err(ActionError::failed("the client stopped receiving"));
        };

        let credit = Credit::decode(&data)
            .ok_or_else(|| ActionError::failed("the client sent a malformed credit"))?;
        self.allowed = self.allowed.max(credit.0);

        Ok(())
    }

    async fn forward(&self, data: Bytes) -> Result<(), ActionError> {
        let frame = Frame::Stream {
            stream: self.stream,
            data,
        };

        let sent = time::timeout(SILENCE_LIMIT, self.frames.send(frame)).await;

        match sent {
            Ok(Ok(())) => Ok(()),
            _ => Err(ActionError::failed("the client is gone")),
        }
    }
}

impl Drop for Outbound {
    fn drop(&mut self) {
        self.link.forget(self.stream);
    }
}
