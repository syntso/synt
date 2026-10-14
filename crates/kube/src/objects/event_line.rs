use k8s_openapi::api::core::v1::Event;
use serde::Serialize;

use super::ObjectRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventLine {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub object: ObjectRef,
    pub count: Option<i32>,
    pub last_seen: Option<String>,
}

impl From<Event> for EventLine {
    fn from(event: Event) -> Self {
        let involved = event.involved_object;
        let last_seen = event
            .last_timestamp
            .map(|time| time.0.to_string())
            .or_else(|| event.event_time.map(|time| time.0.to_string()))
            .or_else(|| event.first_timestamp.map(|time| time.0.to_string()));

        Self {
            kind: event.type_,
            reason: event.reason,
            message: event.message,
            object: ObjectRef {
                kind: involved.kind.unwrap_or_default(),
                name: involved.name.unwrap_or_default(),
                namespace: involved.namespace,
            },
            count: event.count,
            last_seen,
        }
    }
}
