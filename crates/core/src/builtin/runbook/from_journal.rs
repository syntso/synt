use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::{Event, EventKind, JournalReader, Seq};

use super::Drafted;
use crate::CoreError;
use crate::builtin::blocking::blocking;
use crate::runbooks::{Draft, DraftStep, Runbook, Runbooks};

const MOST_EVENTS: usize = 100;
const DEFAULT_NAME: &str = "Steps from the journal";
const COMMAND: &str = "command";

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

        let runbook = blocking(move || {
            let reader = journal.reader()?;
            drafted(&reader, &params.events, name)
        })
        .await?;

        Drafted::new(runbook)
    }
}

fn drafted(reader: &JournalReader, events: &[u64], name: String) -> Result<Runbook, CoreError> {
    let mut draft = Draft::new(name);
    let mut seen = BTreeSet::new();

    for seq in events {
        let request = requested(reader, Seq::new(*seq))?;

        if seen.insert(request.seq()) {
            draft.push(step(&request));
        }
    }

    draft.build().map_err(CoreError::Runbook)
}

fn requested(reader: &JournalReader, seq: Seq) -> Result<Event, CoreError> {
    let event = find(reader, seq)?;

    if event.kind() == EventKind::Requested {
        return Ok(event);
    }

    let Some(call) = event.call() else {
        return Err(CoreError::Runbook(format!(
            "journal event {seq} belongs to no call"
        )));
    };

    let request = find(reader, call)?;

    if request.kind() != EventKind::Requested {
        return Err(CoreError::Runbook(format!(
            "journal event {call} is not a request"
        )));
    }

    Ok(request)
}

fn find(reader: &JournalReader, seq: Seq) -> Result<Event, CoreError> {
    reader
        .get(seq)?
        .ok_or_else(|| CoreError::Runbook(format!("the journal has no event {seq}")))
}

fn step(request: &Event) -> DraftStep {
    let action = request.action().as_str().to_owned();
    let target = request.resource().map(|target| target.as_str().to_owned());
    let params: Map<String, Value> = request
        .details()
        .value()
        .get("params")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let title = match (params.get(COMMAND).and_then(Value::as_str), &target) {
        (Some(command), _) => DraftStep::title_from(command),
        (None, Some(target)) => format!("{action} on {target}"),
        (None, None) => action.clone(),
    };

    DraftStep {
        title,
        action,
        target,
        params,
    }
}
