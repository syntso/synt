use serde::Serialize;
use synt_journal::{Details, EventKind, JournalError, JournalWriter, NewEvent, Seq};

use super::Call;

pub(crate) struct Recorder {
    journal: JournalWriter,
}

impl Recorder {
    pub(crate) fn new(journal: JournalWriter) -> Self {
        Self { journal }
    }

    pub(crate) async fn requested(
        &self,
        call: &Call<'_>,
        details: Details,
    ) -> Result<Seq, JournalError> {
        let event = Self::event(call, EventKind::Requested).details(details);
        self.journal.append_async(event).await
    }

    pub(crate) async fn follow_up(
        &self,
        call: &Call<'_>,
        request: Seq,
        kind: EventKind,
        details: &impl Serialize,
    ) -> Result<Seq, JournalError> {
        let details = Details::from_serialize(details).unwrap_or_default();
        let event = Self::event(call, kind).call(request).details(details);

        self.journal.append_async(event).await
    }

    fn event(call: &Call<'_>, kind: EventKind) -> NewEvent {
        let event = NewEvent::new(call.actor.clone(), kind, call.action.clone());

        match call.target {
            Some(target) => event.resource(target.clone()),
            None => event,
        }
    }
}
