use std::sync::Arc;

use bytes::Bytes;
use synt_actions::{ActionError, Context};
use synt_kube::{KubeError, LogStream, Watch};
use synt_proto::StreamId;
use tokio_util::sync::CancellationToken;

use super::{EndReason, StreamEnd, Tally};
use crate::link::{Link, Outbound};

pub(crate) fn outbound(context: &Context, stream: StreamId) -> Result<Outbound, ActionError> {
    let link: Arc<Link> = context
        .connection::<Link>()
        .ok_or_else(|| ActionError::failed("this call has no connection to stream over"))?;

    Outbound::open(&link, stream)
}

pub(crate) async fn pump_logs(
    mut logs: LogStream,
    mut outbound: Outbound,
    stop: CancellationToken,
) -> StreamEnd {
    let mut tally = Tally::start();

    let ended = loop {
        let next = tokio::select! {
            () = stop.cancelled() => break tally.end(EndReason::Closed, None),
            next = logs.next() => next,
        };

        match next {
            Ok(Some(data)) => {
                let lines = data.split(|byte| *byte == b'\n').count() - 1;
                tally.add(data.len(), lines);

                if outbound.send(data).await.is_err() {
                    break tally.end(EndReason::ClientGone, None);
                }
            }
            Ok(None) => break tally.end(EndReason::Ended, None),
            Err(error) => break failure(&tally, &error),
        }
    };

    outbound.finish().await;
    ended
}

pub(crate) async fn pump_watch(
    mut watch: Watch,
    mut outbound: Outbound,
    stop: CancellationToken,
) -> StreamEnd {
    let mut tally = Tally::start();

    let ended = loop {
        let next = tokio::select! {
            () = stop.cancelled() => break tally.end(EndReason::Closed, None),
            next = watch.next() => next,
        };

        let notice = match next {
            Ok(notice) => notice,
            Err(error) => break failure(&tally, &error),
        };

        let Ok(mut line) = serde_json::to_vec(&notice) else {
            continue;
        };
        line.push(b'\n');
        tally.add(line.len(), 1);

        if outbound.send(Bytes::from(line)).await.is_err() {
            break tally.end(EndReason::ClientGone, None);
        }
    };

    outbound.finish().await;
    ended
}

fn failure(tally: &Tally, error: &KubeError) -> StreamEnd {
    tally.end(EndReason::Failed, Some(error.to_string()))
}
