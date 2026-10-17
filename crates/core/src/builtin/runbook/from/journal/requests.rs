use std::collections::BTreeSet;

use serde_json::{Map, Value};
use synt_journal::{Event, EventKind, JournalReader, Seq};
use synt_proto::OneLine;

use crate::CoreError;
use crate::runbooks::{Draft, DraftStep, RunbookDocument, RunbookError};

const PARAMS: &str = "params";

pub(super) struct Requests<'a> {
    reader: &'a JournalReader,
}

impl<'a> Requests<'a> {
    pub(super) fn new(reader: &'a JournalReader) -> Self {
        Self { reader }
    }

    pub(super) fn draft(
        &self,
        events: &[u64],
        name: OneLine,
    ) -> Result<RunbookDocument, CoreError> {
        let mut draft = Draft::new(name);
        let mut seen = BTreeSet::new();

        for seq in events {
            let request = self.request_of(Seq::new(*seq))?;

            if seen.insert(request.seq()) {
                draft.push(Self::step(&request));
            }
        }

        Ok(draft.build()?)
    }

    fn request_of(&self, seq: Seq) -> Result<Event, CoreError> {
        let event = self.find(seq)?;

        if event.kind() == EventKind::Requested {
            return Ok(event);
        }

        let call = event.call().ok_or(RunbookError::NoCall(seq))?;
        let request = self.find(call)?;

        if request.kind() != EventKind::Requested {
            return Err(RunbookError::NotARequest(call).into());
        }

        Ok(request)
    }

    fn find(&self, seq: Seq) -> Result<Event, CoreError> {
        let event = self.reader.get(seq)?;

        Ok(event.ok_or(RunbookError::NoEvent(seq))?)
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
