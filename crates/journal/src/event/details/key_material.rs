use serde_json::Value;

const MARKERS: [&str; 3] = [
    "PRIVATE KEY-----",
    "PuTTY-User-Key-File-",
    "-----BEGIN PGP PRIVATE",
];

pub(super) struct KeyMaterial;

impl KeyMaterial {
    pub(super) fn found_in(value: &Value) -> bool {
        match value {
            Value::String(text) => Self::marks(text),
            Value::Array(items) => items.iter().any(Self::found_in),
            Value::Object(entries) => entries
                .iter()
                .any(|(key, value)| Self::marks(key) || Self::found_in(value)),
            Value::Null | Value::Bool(_) | Value::Number(_) => false,
        }
    }

    fn marks(text: &str) -> bool {
        MARKERS.iter().any(|marker| text.contains(marker))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::KeyMaterial;

    #[test]
    fn finds_pem_and_openssh_keys_anywhere() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let openssh = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3Bl...";

        assert!(KeyMaterial::found_in(&json!({ "output": pem })));
        assert!(KeyMaterial::found_in(&json!([1, { "nested": [openssh] }])));
        assert!(KeyMaterial::found_in(&json!({ (pem): 1 })));
    }

    #[test]
    fn leaves_ordinary_details_alone() {
        let details = json!({ "command": "cat ~/.ssh/id_ed25519.pub", "exit": 0 });
        assert!(!KeyMaterial::found_in(&details));
    }
}
