mod call;
mod rejection;

pub(crate) use call::Call;
pub(crate) use rejection::Rejection;

use synt_journal::{JournalError, JournalWriter, NewEvent, Seq};

pub(crate) struct Recorder {
    journal: JournalWriter,
}

impl Recorder {
    pub(crate) fn new(journal: JournalWriter) -> Self {
        Self { journal }
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
}
