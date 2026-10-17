mod chunk;
mod close;
mod end;
mod entry;
mod feed;
mod pump;
mod started;
mod tally;

use chunk::Chunk;
pub(crate) use close::K8sStreamClose;
use end::{EndReason, StreamEnd};
use entry::StreamEntry;
use feed::Feed;
pub(crate) use pump::Pump;
pub(crate) use started::StreamStarted;
use tally::Tally;

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;
use synt_actions::Context;
use synt_journal::{ActionId, Actor, EventKind, JournalWriter, Seq};
use synt_platform::Lock;
use tokio_util::sync::CancellationToken;

use super::K8sError;
use crate::log::log;
use crate::recorder::{Call, Recorder};

type Open = BTreeMap<Seq, StreamEntry>;

#[derive(Clone)]
pub(crate) struct Streams {
    open: Arc<Mutex<Open>>,
    recorder: Arc<Recorder>,
}

impl Streams {
    pub(crate) fn new(journal: JournalWriter) -> Self {
        Self {
            open: Arc::default(),
            recorder: Arc::new(Recorder::new(journal)),
        }
    }

    pub(crate) fn start<Work, Running>(
        &self,
        context: &Context,
        action: &'static str,
        about: Value,
        work: Work,
    ) where
        Work: FnOnce(CancellationToken) -> Running + Send + 'static,
        Running: Future<Output = StreamEnd> + Send + 'static,
    {
        let stop = context.stop().child_token();
        let request = context.request();
        let entry = StreamEntry::new(context.actor().clone(), stop.clone());
        self.lock().insert(request, entry);

        let streams = self.clone();
        let context = context.clone();

        tokio::spawn(async move {
            let ended = work(stop).await;

            streams.lock().remove(&request);
            streams.record(&context, action, about, &ended).await;
        });
    }

    pub(crate) fn close(&self, request: Seq, actor: &Actor) -> Result<(), K8sError> {
        let open = self.lock();
        let stream = open.get(&request).ok_or(K8sError::NoStream(request))?;

        if !stream.may_be_closed_by(actor) {
            return Err(K8sError::StreamOfAnother(request));
        }

        stream.close();

        Ok(())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.lock().is_empty()
    }

    async fn record(
        &self,
        context: &Context,
        action: &'static str,
        about: Value,
        ended: &StreamEnd,
    ) {
        let Ok(action) = ActionId::new(action) else {
            return;
        };

        let call = Call {
            actor: context.actor(),
            action: &action,
            target: context.target(),
        };
        let finished = call.step(EventKind::Finished, &ended.summary(about));

        if let Err(error) = self
            .recorder
            .follow_up(context.request(), vec![finished])
            .await
        {
            log!(
                "journal: the stream of request {} ended but was not journaled: {error}",
                context.request()
            );
        }
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        self.open.locked()
    }
}

impl Debug for Streams {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Streams")
            .field("open", &self.lock().len())
            .finish_non_exhaustive()
    }
}
