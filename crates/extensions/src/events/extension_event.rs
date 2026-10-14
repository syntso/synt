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

    pub(crate) fn caused_by(&self, extension: &str) -> bool {
        matches!(&self.actor, Some(Actor::Plugin(name)) if name.as_str() == extension)
    }
}
