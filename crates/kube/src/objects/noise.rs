use serde_json::{Map, Value};

const NOISE: [&str; 5] = [
    "managedFields",
    "resourceVersion",
    "generation",
    "uid",
    "creationTimestamp",
];

pub(crate) trait Noise {
    #[must_use]
    fn without_noise(self) -> Self;
}

impl Noise for Value {
    fn without_noise(mut self) -> Self {
        if let Some(fields) = self.as_object_mut() {
            fields.remove("status");

            if let Some(metadata) = fields.get_mut("metadata").and_then(Value::as_object_mut) {
                strip_metadata(metadata);
            }
        }

        self
    }
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

    use super::Noise;

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
            object.without_noise(),
            json!({
                "kind": "ConfigMap",
                "metadata": { "name": "settings", "labels": { "app": "orders" } },
                "data": { "mode": "fast" }
            })
        );
    }
}
