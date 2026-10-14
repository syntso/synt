use std::collections::BTreeMap;
use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::{Map, Value};
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

    pub(crate) fn start<Pump, Running>(
        &self,
        context: &Context,
        action: &'static str,
        about: Value,
        pump: Pump,
    ) where
        Pump: FnOnce(CancellationToken) -> Running + Send + 'static,
        Running: Future<Output = StreamEnd> + Send + 'static,
    {
        let stop = context.stop().child_token();
        let request = context.request();
        let open = OpenStream {
            actor: context.actor().clone(),
            stop: stop.clone(),
        };
        self.lock().insert(request, open);

        let streams = self.clone();
        let context = context.clone();

        tokio::spawn(async move {
            let ended = pump(stop).await;

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

        if stream.actor != *actor && *actor != Actor::Person {
            return Err(ActionError::Failed(format!(
                "the stream of request {request} belongs to another caller"
            )));
        }

        stream.stop.cancel();

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
        let finished = Recorder::step(&call, EventKind::Finished, &summary(about, ended));

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

fn summary(about: Value, ended: &StreamEnd) -> Value {
    let mut fields = match about {
        Value::Object(fields) => fields,
        _ => Map::new(),
    };

    if let Ok(Value::Object(end)) = serde_json::to_value(ended) {
        fields.extend(end);
    }

    Value::Object(fields)
}
