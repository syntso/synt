use serde_json::Value;
use synt_journal::Actor;

use crate::manifest::EventTopic;

#[derive(Debug, Clone, PartialEq)]
pub struct ExtensionEvent {
    pub topic: EventTopic,
    pub payload: Value,
    pub actor: Option<Actor>,
}

impl ExtensionEvent {
    pub fn new(topic: EventTopic, payload: Value) -> Self {
        Self {
            topic,
            payload,
            actor: None,
        }
    }

    #[must_use]
    pub fn by(mut self, actor: Actor) -> Self {
        self.actor = Some(actor);
        self
    }

    pub(crate) fn caused_by_an_extension(&self) -> bool {
        matches!(&self.actor, Some(Actor::Plugin(_)))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use synt_journal::{Actor, Name};

    use super::ExtensionEvent;
    use crate::manifest::EventTopic;

    #[test]
    fn knows_when_an_extension_caused_it() {
        let event = ExtensionEvent::new(EventTopic::Journal, json!({}));
        let by_agent = event.clone().by(Actor::Agent(Name::new("bot").unwrap()));
        let by_extension = event.clone().by(Actor::Plugin(Name::new("other").unwrap()));

        assert!(!event.caused_by_an_extension());
        assert!(!by_agent.caused_by_an_extension());
        assert!(by_extension.caused_by_an_extension());
    }
}
