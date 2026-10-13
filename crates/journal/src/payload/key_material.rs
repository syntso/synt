use serde_json::Value;

const MARKERS: [&str; 3] = [
    "PRIVATE KEY-----",
    "PuTTY-User-Key-File-",
    "-----BEGIN PGP PRIVATE",
];

pub(super) fn contains_key_material(value: &Value) -> bool {
    match value {
        Value::String(text) => looks_like_key(text),
        Value::Array(items) => items.iter().any(contains_key_material),
        Value::Object(entries) => entries
            .iter()
            .any(|(key, value)| looks_like_key(key) || contains_key_material(value)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn looks_like_key(text: &str) -> bool {
    MARKERS.iter().any(|marker| text.contains(marker))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::contains_key_material;

    #[test]
    fn finds_pem_and_openssh_keys_anywhere() {
        let pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let openssh = "-----BEGIN OPENSSH PRIVATE KEY-----\nb3Bl...";

        assert!(contains_key_material(&json!({ "output": pem })));
        assert!(contains_key_material(&json!([1, { "nested": [openssh] }])));
        assert!(contains_key_material(&json!({ (pem): 1 })));
    }

    #[test]
    fn leaves_ordinary_details_alone() {
        let details = json!({ "command": "cat ~/.ssh/id_ed25519.pub", "exit": 0 });
        assert!(!contains_key_material(&details));
    }
}
