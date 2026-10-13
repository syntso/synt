use synt_journal::{Actor, Name};

const PERSON: &str = "person";
const CORE: &str = "core";
const AGENT: &str = "agent:";
const RUNBOOK: &str = "runbook:";
const PLUGIN: &str = "plugin:";

pub(super) fn actor_text(actor: &Actor) -> String {
    match actor {
        Actor::Person => PERSON.to_owned(),
        Actor::Core => CORE.to_owned(),
        Actor::Agent(name) => format!("{AGENT}{name}"),
        Actor::Runbook(name) => format!("{RUNBOOK}{name}"),
        Actor::Plugin(name) => format!("{PLUGIN}{name}"),
    }
}

pub(super) fn parse_actor(text: &str) -> Option<Actor> {
    match text {
        PERSON => return Some(Actor::Person),
        CORE => return Some(Actor::Core),
        _ => {}
    }

    let named = |prefix: &str| {
        text.strip_prefix(prefix)
            .and_then(|name| Name::new(name).ok())
    };

    if let Some(name) = named(AGENT) {
        return Some(Actor::Agent(name));
    }
    if let Some(name) = named(RUNBOOK) {
        return Some(Actor::Runbook(name));
    }

    named(PLUGIN).map(Actor::Plugin)
}
