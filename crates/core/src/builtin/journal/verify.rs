use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::{JournalReader, JournalWriter, Verification};

use super::read::read;

#[derive(Clone)]
pub struct JournalVerify {
    journal: JournalWriter,
}

impl JournalVerify {
    pub fn new(journal: JournalWriter) -> Self {
        Self { journal }
    }
}

impl Action for JournalVerify {
    const ID: &'static str = "journal.verify";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Verification;

    async fn run(&self, _context: &Context, (): ()) -> Result<Verification, ActionError> {
        read(&self.journal, JournalReader::verify).await
    }
}
