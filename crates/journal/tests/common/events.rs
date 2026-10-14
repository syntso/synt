use serde_json::json;
use synt_journal::{ActionId, Actor, Details, EventKind, NewEvent, ResourceRef};

pub fn exec(command: &str) -> NewEvent {
    NewEvent::new(Actor::Person, EventKind::Requested, action("ssh.exec"))
        .resource(ResourceRef::new("@host/api-1").unwrap())
        .details(Details::from_serialize(&json!({ "command": command })).unwrap())
}

pub fn action(id: &str) -> ActionId {
    ActionId::new(id).unwrap()
}
