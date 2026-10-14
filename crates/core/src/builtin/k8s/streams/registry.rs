use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;
use synt_actions::{ActionError, Context};
use synt_journal::{ActionId, Actor, EventKind, JournalWriter, Seq};
use tokio_util::sync::CancellationToken;

use super::{OpenStream, StreamEnd};
use crate::locked::locked;
use crate::recorder::{Call, Recorder};

type Open = BTreeMap<Seq, OpenStream>;

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
        let open = OpenStream::new(context.actor().clone(), stop.clone());
        self.lock().insert(request, open);

        let streams = self.clone();
        let context = context.clone();

        tokio::spawn(async move {
            let ended = work(stop).await;

            streams.lock().remove(&request);
            streams.record(&context, action, about, &ended).await;
        });
    }

    pub(crate) fn close(&self, request: Seq, actor: &Actor) -> Result<(), ActionError> {
        let open = self.lock();

        let Some(stream) = open.get(&request) else {
            return Err(ActionError::Failed(format!(
                "no stream was started by request {request}"
            )));
        };

        if !stream.may_be_closed_by(actor) {
            return Err(ActionError::Failed(format!(
                "the stream of request {request} belongs to another caller"
            )));
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
        let finished = Recorder::step(&call, EventKind::Finished, &ended.summary(about));

        if let Err(error) = self
            .recorder
            .follow_up(context.request(), vec![finished])
            .await
        {
            eprintln!(
                "synt-core: journal: the stream of request {} ended but was not journaled: {error}",
                context.request()
            );
        }
    }

    fn lock(&self) -> MutexGuard<'_, Open> {
        locked(&self.open)
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
