mod journal_entry;

use serde_json::json;
use synt_extensions::{EventTopic, ExtensionEvent, Extensions};
use synt_journal::{Appended, JournalWriter};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::projects::{ProjectChange, Projects};
use journal_entry::JournalEntry;

#[derive(Clone)]
pub struct EventFeed {
    extensions: Extensions,
}

impl EventFeed {
    pub fn start(
        extensions: &Extensions,
        journal: &JournalWriter,
        projects: &Projects,
    ) -> [JoinHandle<()>; 2] {
        let feed = Self {
            extensions: extensions.clone(),
        };

        let from_journal = tokio::spawn(feed.clone().follow_journal(journal.watch()));
        let from_projects = tokio::spawn(feed.follow_projects(projects.changes()));

        [from_journal, from_projects]
    }

    async fn follow_journal(self, mut appended: Receiver<Appended>) {
        loop {
            match appended.recv().await {
                Ok(event) => self.publish(JournalEntry::new(&event).events()),
                Err(RecvError::Lagged(missed)) => {
                    eprintln!("synt-core: extensions missed {missed} journal events");
                }
                Err(RecvError::Closed) => return,
            }
        }
    }

    async fn follow_projects(self, mut changes: Receiver<ProjectChange>) {
        loop {
            match changes.recv().await {
                Ok(change) => self.publish([Self::resource_event(&change)]),
                Err(RecvError::Lagged(missed)) => {
                    eprintln!("synt-core: extensions missed {missed} project changes");
                }
                Err(RecvError::Closed) => return,
            }
        }
    }

    fn resource_event(change: &ProjectChange) -> ExtensionEvent {
        let payload = json!({ "project": change.project, "change": change.change });

        ExtensionEvent::new(EventTopic::Resource, payload)
    }

    fn publish(&self, events: impl IntoIterator<Item = ExtensionEvent>) {
        for event in events {
            self.extensions.publish(&event);
        }
    }
}
