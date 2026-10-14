use serde_json::json;
use synt_journal::{ActionId, Actor, Details, EventKind, JournalWriter, NewEvent};

use crate::id::ProjectId;
use crate::report::Applied;

const APPLY_NOTE: &str = "sync.apply";

pub(crate) struct Notes {
    journal: Option<JournalWriter>,
}

impl Notes {
    pub(crate) fn new(journal: Option<JournalWriter>) -> Self {
        Self { journal }
    }

    pub(crate) fn applied(&self, project: &ProjectId, applied: &[Applied]) {
        let Some(journal) = &self.journal else {
            return;
        };

        for change in applied {
            if let Err(error) = Self::note(journal, project, change) {
                eprintln!("synt-sync: cannot journal {}: {error}", change.path);
            }
        }
    }

    fn note(journal: &JournalWriter, project: &ProjectId, change: &Applied) -> Result<(), String> {
        let details = json!({
            "project": project,
            "path": change.path,
            "revision": change.revision,
            "author": change.author,
            "removed": change.removed,
            "conflicts": change.conflicts,
        });
        let details = Details::from_serialize(&details).map_err(|error| error.to_string())?;
        let action = ActionId::new(APPLY_NOTE).map_err(|error| error.to_string())?;

        let event = NewEvent::new(Actor::Core, EventKind::Note, action).details(details);
        journal.append(event).map_err(|error| error.to_string())?;

        Ok(())
    }
}
