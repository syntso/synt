use crate::model::Actor;
use crate::payload::Details;

pub fn decode_details(bytes: &[u8]) -> Option<Details> {
    Details::decode(bytes.to_vec()).ok()
}

pub fn details_bytes(details: &Details) -> &[u8] {
    details.bytes()
}

pub fn decode_actor(bytes: &[u8]) -> Option<Actor> {
    Actor::decode(bytes).ok()
}

pub fn actor_bytes(actor: &Actor) -> Vec<u8> {
    actor.encode()
}
