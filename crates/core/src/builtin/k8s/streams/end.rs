use serde::Serialize;
use serde_json::{Map, Value};

use super::EndReason;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StreamEnd {
    pub(crate) reason: EndReason,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,

    pub(crate) bytes: u64,
    pub(crate) items: u64,
    pub(crate) duration_ms: u64,
}

impl StreamEnd {
    pub(crate) fn summary(&self, about: Value) -> Value {
        let mut fields = match about {
            Value::Object(fields) => fields,
            _ => Map::new(),
        };

        if let Ok(Value::Object(end)) = serde_json::to_value(self) {
            fields.extend(end);
        }

        Value::Object(fields)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::StreamEnd;
    use crate::builtin::k8s::streams::EndReason;

    #[test]
    fn the_summary_adds_how_the_stream_ended_to_what_it_was_about() {
        let ended = StreamEnd {
            reason: EndReason::ClientGone,
            error: None,
            bytes: 12,
            items: 2,
            duration_ms: 40,
        };

        let summary = ended.summary(json!({ "pod": "api", "stream": 3 }));

        assert_eq!(
            summary,
            json!({
                "pod": "api",
                "stream": 3,
                "reason": "client_gone",
                "bytes": 12,
                "items": 2,
                "duration_ms": 40
            })
        );
    }
}
