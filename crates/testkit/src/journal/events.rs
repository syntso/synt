use serde_json::json;
use synt_journal::{ActionId, Actor, Details, EventKind, NewEvent, ResourceRef};

use crate::TestError;

pub fn exec(command: &str) -> Result<NewEvent, TestError> {
    let resource = ResourceRef::new("@host/api-1")?;
    let details = Details::from_serialize(&json!({ "command": command }))?;

    let event = NewEvent::new(Actor::Person, EventKind::Requested, action("ssh.exec")?)
        .resource(resource)
        .details(details);

    Ok(event)
}

pub fn action(id: &str) -> Result<ActionId, TestError> {
    Ok(ActionId::new(id)?)
}
