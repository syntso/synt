use serde::{Deserialize, Serialize};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::{Event, Filter, JournalWriter, Seq};

use super::read::read;

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 500;

#[derive(Clone)]
pub struct JournalPage {
    journal: JournalWriter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct PageQuery {
    #[serde(default)]
    pub after: Option<u64>,

    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    pub events: Vec<Event>,
}

impl JournalPage {
    pub fn new(journal: JournalWriter) -> Self {
        Self { journal }
    }
}

impl Action for JournalPage {
    const ID: &'static str = "journal.page";
    const RISK: Risk = Risk::Read;

    type Params = Option<PageQuery>;
    type Output = Page;

    async fn run(&self, _context: &Context, query: Option<PageQuery>) -> Result<Page, ActionError> {
        let query = query.unwrap_or_default();
        let limit = checked_limit(query.limit)?;
        let after = query.after.map(Seq::new);

        let events = read(&self.journal, move |reader| {
            reader.page(&Filter::all(), after, limit)
        })
        .await?;

        Ok(Page { events })
    }
}

fn checked_limit(limit: Option<u32>) -> Result<u32, ActionError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);

    if limit == 0 || limit > MAX_LIMIT {
        return Err(ActionError::InvalidParams(format!(
            "limit must be between 1 and {MAX_LIMIT}, got {limit}"
        )));
    }

    Ok(limit)
}
