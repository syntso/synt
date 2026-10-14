use std::collections::BTreeSet;

use serde_json::{Map, Value};
use synt_journal::{Event, EventKind, JournalReader, Seq};

use crate::CoreError;
use crate::runbooks::{Draft, DraftStep, Runbook};

const PARAMS: &str = "params";

pub(super) struct Requests<'a> {
    reader: &'a JournalReader,
}

impl<'a> Requests<'a> {
    pub(super) fn new(reader: &'a JournalReader) -> Self {
        Self { reader }
    }

    pub(super) fn draft(&self, events: &[u64], name: String) -> Result<Runbook, CoreError> {
        let mut draft = Draft::new(name);
        let mut seen = BTreeSet::new();

        for seq in events {
            let request = self.request_of(Seq::new(*seq))?;

            if seen.insert(request.seq()) {
                draft.push(Self::step(&request));
            }
        }

        draft.build().map_err(CoreError::Runbook)
    }

    fn request_of(&self, seq: Seq) -> Result<Event, CoreError> {
        let event = self.find(seq)?;

        if event.kind() == EventKind::Requested {
            return Ok(event);
        }

        let Some(call) = event.call() else {
            return Err(CoreError::Runbook(format!(
                "journal event {seq} belongs to no call"
            )));
        };

        let request = self.find(call)?;

        if request.kind() != EventKind::Requested {
            return Err(CoreError::Runbook(format!(
                "journal event {call} is not a request"
            )));
        }

        Ok(request)
    }

    fn find(&self, seq: Seq) -> Result<Event, CoreError> {
        self.reader
            .get(seq)?
            .ok_or_else(|| CoreError::Runbook(format!("the journal has no event {seq}")))
    }

    fn step(request: &Event) -> DraftStep {
        let action = request.action().as_str().to_owned();
        let target = request.resource().map(|target| target.as_str().to_owned());
        let params: Map<String, Value> = request
            .details()
            .value()
            .get(PARAMS)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        DraftStep::new(action, target, params)
    }
}
