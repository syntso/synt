use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use super::Drafted;
use super::requests::Requests;
use crate::CoreError;
use crate::blocking::blocking;
use crate::runbooks::Runbooks;

const MOST_EVENTS: usize = 100;
const DEFAULT_NAME: &str = "Steps from the journal";

#[derive(Debug, Clone)]
pub struct RunbookFromJournal {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FromJournalParams {
    pub events: Vec<u64>,

    #[serde(default)]
    pub name: Option<String>,
}

impl RunbookFromJournal {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookFromJournal {
    const ID: &'static str = "runbook.from_journal";
    const TITLE: &'static str = "Draft a runbook from calls in the journal";
    const RISK: Risk = Risk::Read;

    type Params = FromJournalParams;
    type Output = Drafted;

    fn summary(&self, output: &Drafted) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: FromJournalParams,
    ) -> Result<Drafted, ActionError> {
        if params.events.is_empty() || params.events.len() > MOST_EVENTS {
            return Err(ActionError::InvalidParams(format!(
                "events must list 1 to {MOST_EVENTS} journal numbers"
            )));
        }

        let journal = self.runbooks.journal().clone();
        let name = params.name.unwrap_or_else(|| DEFAULT_NAME.to_owned());

        let runbook = blocking::<_, CoreError>(move || {
            let reader = journal.reader()?;

            Requests::new(&reader).draft(&params.events, name)
        })
        .await?;

        Drafted::new(runbook)
    }
}
