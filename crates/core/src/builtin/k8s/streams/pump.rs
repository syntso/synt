use std::sync::Arc;

use synt_actions::{ActionError, Context};
use synt_proto::StreamId;
use tokio_util::sync::CancellationToken;

use super::{EndReason, Feed, StreamEnd, Tally};
use crate::link::{Link, Outbound};

pub(crate) struct Pump {
    outbound: Outbound,
}

impl Pump {
    pub(crate) fn open(context: &Context, stream: StreamId) -> Result<Self, ActionError> {
        let link: Arc<Link> = context
            .connection::<Link>()
            .ok_or_else(|| ActionError::failed("this call has no connection to stream over"))?;

        let outbound = Outbound::open(&link, stream)?;

        Ok(Self { outbound })
    }

    pub(crate) async fn run(mut self, mut feed: impl Feed, stop: CancellationToken) -> StreamEnd {
        let mut tally = Tally::start();

        let ended = loop {
            let next = tokio::select! {
                () = stop.cancelled() => break tally.end(EndReason::Closed, None),
                next = feed.next_chunk() => next,
            };

            let chunk = match next {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break tally.end(EndReason::Ended, None),
                Err(error) => break tally.end(EndReason::Failed, Some(error.to_string())),
            };

            tally.add(chunk.data.len(), chunk.items);

            if self.outbound.send(chunk.data).await.is_err() {
                break tally.end(EndReason::ClientGone, None);
            }
        };

        self.outbound.finish().await;

        ended
    }
}
