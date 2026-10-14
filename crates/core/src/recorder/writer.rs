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

    pub(crate) fn request(call: &Call<'_>, details: Details) -> NewEvent {
        Self::event(call, EventKind::Requested).details(details)
    }

    pub(crate) fn step(call: &Call<'_>, kind: EventKind, details: &impl Serialize) -> NewEvent {
        let details = Details::from_serialize(details).unwrap_or_default();

        Self::event(call, kind).details(details)
    }

    pub(crate) async fn open(
        &self,
        request: NewEvent,
        follow_ups: Vec<NewEvent>,
    ) -> Result<Seq, JournalError> {
        self.journal.open_call(request, follow_ups).await
    }

    pub(crate) async fn follow_up(
        &self,
        request: Seq,
        steps: Vec<NewEvent>,
    ) -> Result<(), JournalError> {
        let steps = steps.into_iter().map(|step| step.call(request)).collect();

        self.journal.append_together(steps).await.map(|_| ())
    }

    fn event(call: &Call<'_>, kind: EventKind) -> NewEvent {
        let event = NewEvent::new(call.actor.clone(), kind, call.action.clone());

        match call.target {
            Some(target) => event.resource(target.clone()),
            None => event,
        }
    }
}
