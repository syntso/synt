use serde_json::json;
use synt_extensions::{EventTopic, ExtensionEvent};
use synt_journal::{Appended, EventKind};

const SESSION_OPENERS: [&str; 2] = ["ssh.session.open", "local.session.open"];
const RESOURCE_CHANGERS: [&str; 2] = ["resource.put", "resource.remove"];

pub(super) struct JournalEntry<'a> {
    event: &'a Appended,
}

impl<'a> JournalEntry<'a> {
    pub(super) fn new(event: &'a Appended) -> Self {
        Self { event }
    }

    pub(super) fn events(&self) -> Vec<ExtensionEvent> {
        let heard = [
            Some(self.journal()),
            self.approval(),
            self.session(),
            self.resource(),
        ];

        heard
            .into_iter()
            .flatten()
            .map(|heard| heard.by(self.event.actor.clone()))
            .collect()
    }

    fn journal(&self) -> ExtensionEvent {
        let event = self.event;
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

    fn approval(&self) -> Option<ExtensionEvent> {
        let event = self.event;
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

    fn session(&self) -> Option<ExtensionEvent> {
        if !self.finished(&SESSION_OPENERS) {
            return None;
        }

        let event = self.event;
        let payload = json!({
            "session": event.call,
            "action": event.action,
            "resource": event.resource,
        });

        Some(ExtensionEvent::new(EventTopic::Session, payload))
    }

    fn resource(&self) -> Option<ExtensionEvent> {
        if !self.finished(&RESOURCE_CHANGERS) {
            return None;
        }

        let payload = json!({ "resource": self.event.resource, "action": self.event.action });

        Some(ExtensionEvent::new(EventTopic::Resource, payload))
    }

    fn finished(&self, actions: &[&str]) -> bool {
        self.event.kind == EventKind::Finished && actions.contains(&self.event.action.as_str())
    }
}

#[cfg(test)]
mod tests {
    use synt_extensions::EventTopic;
    use synt_journal::{ActionId, Actor, Appended, EventKind, Name, ResourceRef, Seq};

    use super::JournalEntry;

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
        JournalEntry::new(&appended(kind, action))
            .events()
            .into_iter()
            .map(|event| event.topic())
            .collect()
    }

    #[test]
    fn every_entry_is_a_journal_event_by_its_actor() {
        let heard = JournalEntry::new(&appended(EventKind::Started, "ssh.exec")).events();

        assert_eq!(heard.len(), 1);
        assert_eq!(heard[0].topic(), EventTopic::Journal);
        assert_eq!(heard[0].payload()["seq"], 9);
        assert_eq!(heard[0].payload()["resource"], "@orders/api-1");
        assert_eq!(
            heard[0].actor(),
            Some(&Actor::Agent(Name::new("bot").unwrap()))
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
