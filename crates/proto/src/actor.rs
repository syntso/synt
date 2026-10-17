use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{Name, ProtoError};

const PERSON: &str = "person";
const CORE: &str = "core";
const AGENT: &str = "agent:";
const RUNBOOK: &str = "runbook:";
const PLUGIN: &str = "plugin:";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "name", rename_all = "snake_case")]
pub enum Actor {
    Person,
    Core,
    Agent(Name),
    Runbook(Name),
    Plugin(Name),
}

impl Actor {
    pub fn acts_for_person(&self) -> bool {
        matches!(self, Self::Person | Self::Core)
    }

    fn named(text: &str) -> Result<Self, ProtoError> {
        if let Some(name) = text.strip_prefix(AGENT) {
            return Name::new(name).map(Self::Agent);
        }
        if let Some(name) = text.strip_prefix(RUNBOOK) {
            return Name::new(name).map(Self::Runbook);
        }
        if let Some(name) = text.strip_prefix(PLUGIN) {
            return Name::new(name).map(Self::Plugin);
        }

        Err(ProtoError::UnknownActor(text.to_owned()))
    }
}

impl fmt::Display for Actor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Person => f.write_str(PERSON),
            Self::Core => f.write_str(CORE),
            Self::Agent(name) => write!(f, "{AGENT}{name}"),
            Self::Runbook(name) => write!(f, "{RUNBOOK}{name}"),
            Self::Plugin(name) => write!(f, "{PLUGIN}{name}"),
        }
    }
}

impl FromStr for Actor {
    type Err = ProtoError;

    fn from_str(text: &str) -> Result<Self, ProtoError> {
        match text {
            PERSON => Ok(Self::Person),
            CORE => Ok(Self::Core),
            _ => Self::named(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Actor;
    use crate::Name;

    fn agent(name: &str) -> Actor {
        Actor::Agent(Name::new(name).unwrap())
    }

    #[test]
    fn text_round_trips() {
        let actors = [
            Actor::Person,
            Actor::Core,
            agent("deploy-bot"),
            Actor::Runbook(Name::new("restart").unwrap()),
            Actor::Plugin(Name::new("k8s").unwrap()),
        ];

        for actor in actors {
            assert_eq!(actor.to_string().parse::<Actor>().unwrap(), actor);
        }
    }

    #[test]
    fn text_names_the_kind() {
        assert_eq!(agent("deploy-bot").to_string(), "agent:deploy-bot");
        assert_eq!(Actor::Person.to_string(), "person");
    }

    #[test]
    fn unknown_text_is_refused() {
        assert!("robot:x".parse::<Actor>().is_err());
        assert!("agent:".parse::<Actor>().is_err());
    }

    #[test]
    fn json_tags_the_kind() {
        assert_eq!(
            serde_json::to_value(agent("deploy-bot")).unwrap(),
            json!({ "type": "agent", "name": "deploy-bot" })
        );
        assert_eq!(
            serde_json::to_value(Actor::Person).unwrap(),
            json!({ "type": "person" })
        );
    }

    #[test]
    fn only_the_person_and_the_core_act_for_the_person() {
        assert!(Actor::Person.acts_for_person());
        assert!(Actor::Core.acts_for_person());
        assert!(!agent("deploy-bot").acts_for_person());
    }
}
