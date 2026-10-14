use synt_extension_api::serde_json::{self, Value, json};
use synt_extension_api::view::Patch;
use synt_extension_api::{Error, Event, storage, timers, ui};

use crate::params::Params;

const EVENTS: &str = "events";
const STARTED: &str = "started";
const KEPT_EVENTS: usize = 200;

pub fn remember(params: &Params) -> Result<Value, Error> {
    storage::set(params.text("key")?, params.text("value")?.as_bytes())?;

    Ok(json!({ "kept": params.text("key")? }))
}

pub fn recall(params: &Params) -> Result<Value, Error> {
    let found = storage::get(params.text("key")?).map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    Ok(json!({ "value": found }))
}

pub fn arm(params: &Params) -> Result<Value, Error> {
    timers::set(params.text("name")?, params.number("seconds")?)?;

    Ok(json!({ "armed": params.text("name")? }))
}

pub fn disarm(params: &Params) -> Result<Value, Error> {
    timers::clear(params.text("name")?);

    Ok(json!({ "disarmed": params.text("name")? }))
}

pub fn heard() -> Value {
    json!({
        "started": storage::get(STARTED).is_some(),
        "events": read_list(EVENTS),
    })
}

pub fn started() -> Result<(), Error> {
    storage::set(STARTED, b"yes")
}

pub fn hear(event: &Event) -> Result<(), Error> {
    let payload: Value = serde_json::from_str(event.payload()).unwrap_or(Value::Null);
    let mut events = read_list(EVENTS);

    events.push(json!({ "kind": event.kind(), "payload": payload }));
    let start = events.len().saturating_sub(KEPT_EVENTS);

    write_list(EVENTS, &events[start..])
}

pub fn beat(name: &str) -> Result<(), Error> {
    let key = format!("timer:{name}");
    let count = count(&key) + 1;

    storage::set(&key, count.to_string().as_bytes())?;

    let patch = Patch::new().replace("/text", format!("beats: {count}"));
    ui::patch_view("beat", &patch.to_json())
}

pub fn count(key: &str) -> u64 {
    storage::get(key)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| text.parse().ok())
        .unwrap_or(0)
}

fn read_list(key: &str) -> Vec<Value> {
    storage::get(key)
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_list(key: &str, items: &[Value]) -> Result<(), Error> {
    let text = Value::Array(items.to_vec()).to_string();

    storage::set(key, text.as_bytes())
}
