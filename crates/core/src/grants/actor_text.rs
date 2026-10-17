use synt_journal::{Actor, Name};

const PERSON: &str = "person";
const CORE: &str = "core";
const AGENT: &str = "agent:";
const RUNBOOK: &str = "runbook:";
const PLUGIN: &str = "plugin:";

pub(super) struct ActorText(String);

impl ActorText {
    pub(super) fn new(text: String) -> Self {
        Self(text)
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn actor(&self) -> Option<Actor> {
        match self.0.as_str() {
            PERSON => return Some(Actor::Person),
            CORE => return Some(Actor::Core),
            _ => {}
        }

        if let Some(name) = self.named(AGENT) {
            return Some(Actor::Agent(name));
        }

        if let Some(name) = self.named(RUNBOOK) {
            return Some(Actor::Runbook(name));
        }

        self.named(PLUGIN).map(Actor::Plugin)
    }

    fn named(&self, prefix: &str) -> Option<Name> {
        let name = self.0.strip_prefix(prefix)?;

        Name::new(name).ok()
    }
}

impl From<&Actor> for ActorText {
    fn from(actor: &Actor) -> Self {
        let text = match actor {
            Actor::Person => PERSON.to_owned(),
            Actor::Core => CORE.to_owned(),
            Actor::Agent(name) => format!("{AGENT}{name}"),
            Actor::Runbook(name) => format!("{RUNBOOK}{name}"),
            Actor::Plugin(name) => format!("{PLUGIN}{name}"),
        };

        Self(text)
    }
}
