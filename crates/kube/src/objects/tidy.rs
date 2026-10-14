use kube::api::{DynamicObject, TypeMeta};
use serde_json::{Map, Value};

use crate::Kind;

const NOISE: [&str; 5] = [
    "managedFields",
    "resourceVersion",
    "generation",
    "uid",
    "creationTimestamp",
];

pub(crate) fn tidy(mut object: DynamicObject, kind: &Kind) -> Value {
    object.types = Some(TypeMeta {
        api_version: kind.api_version().to_owned(),
        kind: kind.name().to_owned(),
    });
    object.metadata.managed_fields = None;

    serde_json::to_value(object).unwrap_or(Value::Null)
}

pub(crate) fn without_noise(mut object: Value) -> Value {
    if let Some(fields) = object.as_object_mut() {
        fields.remove("status");

        if let Some(metadata) = fields.get_mut("metadata").and_then(Value::as_object_mut) {
            strip_metadata(metadata);
        }
    }

    object
}

fn strip_metadata(metadata: &mut Map<String, Value>) {
    for field in NOISE {
        metadata.remove(field);
    }

    let empty_annotations = metadata
        .get("annotations")
        .and_then(Value::as_object)
        .is_some_and(Map::is_empty);

    if empty_annotations {
        metadata.remove("annotations");
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::without_noise;

    #[test]
    fn noise_the_server_adds_is_left_out_of_a_comparison() {
        let object = json!({
            "kind": "ConfigMap",
            "metadata": {
                "name": "settings",
                "uid": "1f2e",
                "resourceVersion": "812",
                "generation": 3,
                "creationTimestamp": "2026-10-14T09:00:00Z",
                "managedFields": [{ "manager": "synt" }],
                "annotations": {},
                "labels": { "app": "orders" }
            },
            "data": { "mode": "fast" },
            "status": { "phase": "Active" }
        });

        assert_eq!(
            without_noise(object),
            json!({
                "kind": "ConfigMap",
                "metadata": { "name": "settings", "labels": { "app": "orders" } },
                "data": { "mode": "fast" }
            })
        );
    }
}
