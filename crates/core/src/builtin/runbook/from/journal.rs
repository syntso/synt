mod requests;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk, blocking};

use super::Drafted;
use crate::CoreError;
use crate::runbooks::Runbooks;
use requests::Requests;
use synt_proto::OneLine;

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
    pub name: Option<OneLine>,
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
            return Err(CoreError::Picks {
                what: "events",
                most: MOST_EVENTS,
                numbers: "journal numbers",
            }
            .into());
        }

        let journal = self.runbooks.journal().clone();
        let name = match params.name {
            Some(name) => name,
            None => OneLine::new(DEFAULT_NAME).map_err(CoreError::NotOneLine)?,
        };

        let runbook = blocking::<_, _, CoreError>(move || {
            let reader = journal.reader()?;

            Requests::new(&reader).draft(&params.events, name)
        })
        .await?;

        Ok(Drafted::new(runbook)?)
    }
}
