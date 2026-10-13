use serde::Serialize;

use super::Name;
use crate::codec::{CodecError, Decoder, Encoder};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "type", content = "name", rename_all = "snake_case")]
pub enum Actor {
    Person,
    Core,
    Agent(Name),
    Runbook(Name),
    Plugin(Name),
}

const PERSON: u8 = 1;
const CORE: u8 = 2;
const AGENT: u8 = 3;
const RUNBOOK: u8 = 4;
const PLUGIN: u8 = 5;

impl Actor {
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        let mut encoder = Encoder::new(&mut out);

        match self {
            Self::Person => encoder.u8(PERSON),
            Self::Core => encoder.u8(CORE),
            Self::Agent(name) => {
                encoder.u8(AGENT);
                encoder.str(name.as_str());
            }
            Self::Runbook(name) => {
                encoder.u8(RUNBOOK);
                encoder.str(name.as_str());
            }
            Self::Plugin(name) => {
                encoder.u8(PLUGIN);
                encoder.str(name.as_str());
            }
        }

        out
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, CodecError> {
        let mut decoder = Decoder::new(bytes);
        let tag = decoder.u8()?;

        let actor = match tag {
            PERSON => Self::Person,
            CORE => Self::Core,
            AGENT => Self::Agent(Self::name(&mut decoder)?),
            RUNBOOK => Self::Runbook(Self::name(&mut decoder)?),
            PLUGIN => Self::Plugin(Self::name(&mut decoder)?),
            _ => return Err(CodecError("unknown actor")),
        };

        decoder.finish()?;
        Ok(actor)
    }

    fn name(decoder: &mut Decoder<'_>) -> Result<Name, CodecError> {
        let text = decoder.str()?;
        Name::new(text).map_err(|_| CodecError("invalid actor name"))
    }
}

#[cfg(test)]
mod tests {
    use super::Actor;
    use crate::model::Name;

    fn name(text: &str) -> Name {
        Name::new(text).unwrap()
    }

    #[test]
    fn every_actor_round_trips() {
        let actors = [
            Actor::Person,
            Actor::Core,
            Actor::Agent(name("agent-1")),
            Actor::Runbook(name("deploy-api")),
            Actor::Plugin(name("k8s")),
        ];

        for actor in actors {
            assert_eq!(Actor::decode(&actor.encode()), Ok(actor));
        }
    }

    #[test]
    fn different_actors_never_share_bytes() {
        assert_ne!(
            Actor::Agent(name("x")).encode(),
            Actor::Plugin(name("x")).encode()
        );
    }

    #[test]
    fn unknown_tags_are_refused() {
        assert!(Actor::decode(&[99]).is_err());
    }
}
