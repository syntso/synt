use serde_json::json;
use synt_extensions::{EventTopic, ExtensionEvent, Extensions};
use synt_journal::{Appended, EventKind, JournalWriter};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::projects::{ProjectChange, Projects};

const SESSION_OPENERS: [&str; 2] = ["ssh.session.open", "local.session.open"];
const RESOURCE_CHANGERS: [&str; 2] = ["resource.put", "resource.remove"];

pub struct EventFeed;

impl EventFeed {
    pub fn start(
        extensions: &Extensions,
        journal: &JournalWriter,
        projects: &Projects,
    ) -> [JoinHandle<()>; 2] {
        let from_journal = tokio::spawn(follow_journal(extensions.clone(), journal.watch()));
        let from_projects = tokio::spawn(follow_projects(extensions.clone(), projects.changes()));

        [from_journal, from_projects]
    }
}

async fn follow_journal(extensions: Extensions, mut appended: Receiver<Appended>) {
    loop {
        match appended.recv().await {
            Ok(event) => {
                for heard in events_of(&event) {
                    extensions.publish(&heard);
                }
            }
            Err(RecvError::Lagged(missed)) => {
                eprintln!("synt-core: extensions missed {missed} journal events");
            }
            Err(RecvError::Closed) => return,
        }
    }
}

async fn follow_projects(extensions: Extensions, mut changes: Receiver<ProjectChange>) {
    loop {
        match changes.recv().await {
            Ok(change) => {
                let payload = json!({ "project": change.project, "change": change.change });
                extensions.publish(&ExtensionEvent::new(EventTopic::Resource, payload));
            }
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return,
        }
    }
}

pub(crate) fn events_of(event: &Appended) -> Vec<ExtensionEvent> {
    let heard = [
        Some(journal_event(event)),
        approval_event(event),
        session_event(event),
        resource_event(event),
    ];

    heard
        .into_iter()
        .flatten()
        .map(|heard| heard.by(event.actor.clone()))
        .collect()
}

fn journal_event(event: &Appended) -> ExtensionEvent {
    let payload = json!({
        "seq": event.seq,
        "actor": event.actor,
        "kind": event.kind,
        "call": event.call,
        "action": event.action,
        "resource": event.resource,
    });

    ExtensionEvent::new(EventTopic::Journal, payload)
}

fn approval_event(event: &Appended) -> Option<ExtensionEvent> {
    let outcome = match event.kind {
        EventKind::Approved => "approved",
        EventKind::Denied => "denied",
        _ => return None,
    };

    let payload = json!({
        "request": event.call,
        "actor": event.actor,
        "action": event.action,
        "resource": event.resource,
        "outcome": outcome,
    });

    Some(ExtensionEvent::new(EventTopic::Approval, payload))
}

fn session_event(event: &Appended) -> Option<ExtensionEvent> {
    if !finished(event, &SESSION_OPENERS) {
        return None;
    }

    let payload = json!({
        "session": event.call,
        "action": event.action,
        "resource": event.resource,
    });

    Some(ExtensionEvent::new(EventTopic::Session, payload))
}

fn resource_event(event: &Appended) -> Option<ExtensionEvent> {
    if !finished(event, &RESOURCE_CHANGERS) {
        return None;
    }

    let payload = json!({ "resource": event.resource, "action": event.action });

    Some(ExtensionEvent::new(EventTopic::Resource, payload))
}

fn finished(event: &Appended, actions: &[&str]) -> bool {
    event.kind == EventKind::Finished && actions.contains(&event.action.as_str())
}

#[cfg(test)]
mod tests {
    use synt_extensions::EventTopic;
    use synt_journal::{ActionId, Actor, Appended, EventKind, Name, ResourceRef, Seq};

    use super::events_of;

    fn appended(kind: EventKind, action: &str) -> Appended {
        Appended {
            seq: Seq::new(9),
            actor: Actor::Agent(Name::new("bot").unwrap()),
            kind,
            call: Some(Seq::new(7)),
            action: ActionId::new(action).unwrap(),
            resource: Some(ResourceRef::new("@orders/api-1").unwrap()),
        }
    }

    fn topics(kind: EventKind, action: &str) -> Vec<EventTopic> {
        events_of(&appended(kind, action))
            .into_iter()
            .map(|event| event.topic)
            .collect()
    }

    #[test]
    fn every_entry_is_a_journal_event_by_its_actor() {
        let heard = events_of(&appended(EventKind::Started, "ssh.exec"));

        assert_eq!(heard.len(), 1);
        assert_eq!(heard[0].topic, EventTopic::Journal);
        assert_eq!(heard[0].payload["seq"], 9);
        assert_eq!(heard[0].payload["resource"], "@orders/api-1");
        assert_eq!(
            heard[0].actor,
            Some(Actor::Agent(Name::new("bot").unwrap()))
        );
    }

    #[test]
    fn answers_sessions_and_resource_changes_have_topics_of_their_own() {
        assert_eq!(
            topics(EventKind::Denied, "ssh.exec"),
            [EventTopic::Journal, EventTopic::Approval]
        );
        assert_eq!(
            topics(EventKind::Finished, "ssh.session.open"),
            [EventTopic::Journal, EventTopic::Session]
        );
        assert_eq!(
            topics(EventKind::Finished, "resource.put"),
            [EventTopic::Journal, EventTopic::Resource]
        );
        assert_eq!(
            topics(EventKind::Failed, "resource.put"),
            [EventTopic::Journal]
        );
    }
}
