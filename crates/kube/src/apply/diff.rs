use serde_json::{Map, Value};
use similar::TextDiff;

const CONTEXT_LINES: usize = 3;
const HIDDEN: &str = "***";
const HIDDEN_BEFORE: &str = "*** (before)";
const HIDDEN_AFTER: &str = "*** (after)";
const SECRET_FIELDS: [&str; 2] = ["data", "stringData"];

pub(crate) fn unified(label: &str, before: Option<&Value>, after: &Value) -> String {
    let before_text = before.map(yaml).unwrap_or_default();
    let after_text = yaml(after);

    if before_text == after_text {
        return String::new();
    }

    let old_label = if before.is_some() {
        label.to_owned()
    } else {
        "/dev/null".to_owned()
    };

    TextDiff::from_lines(&before_text, &after_text)
        .unified_diff()
        .context_radius(CONTEXT_LINES)
        .header(&old_label, label)
        .to_string()
}

pub(crate) fn hide_secret_values(before: Option<&mut Value>, after: &mut Value) {
    let mut empty = Value::Null;
    let before = before.unwrap_or(&mut empty);

    for field in SECRET_FIELDS {
        let old = take_map(before, field);
        let new = take_map(after, field);

        if let Some(old) = &old {
            put_map(before, field, hidden(old, new.as_ref(), HIDDEN_BEFORE));
        }
        if let Some(new) = &new {
            put_map(after, field, hidden(new, old.as_ref(), HIDDEN_AFTER));
        }
    }
}

fn hidden(
    values: &Map<String, Value>,
    other: Option<&Map<String, Value>>,
    changed: &str,
) -> Map<String, Value> {
    values
        .iter()
        .map(|(key, value)| {
            let same = other.and_then(|other| other.get(key)) == Some(value);
            let shown = if same { HIDDEN } else { changed };

            (key.clone(), Value::String(shown.to_owned()))
        })
        .collect()
}

fn take_map(object: &mut Value, field: &str) -> Option<Map<String, Value>> {
    match object.as_object_mut()?.remove(field)? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

fn put_map(object: &mut Value, field: &str, map: Map<String, Value>) {
    if let Some(fields) = object.as_object_mut() {
        fields.insert(field.to_owned(), Value::Object(map));
    }
}

fn yaml(value: &Value) -> String {
    serde_saphyr::to_string(value).unwrap_or_else(|error| format!("# cannot show: {error}\n"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{hide_secret_values, unified};

    #[test]
    fn a_change_shows_as_a_unified_diff_of_yaml() {
        let before = json!({ "kind": "ConfigMap", "data": { "mode": "slow", "size": "3" } });
        let after = json!({ "kind": "ConfigMap", "data": { "mode": "fast", "size": "3" } });

        let diff = unified("ConfigMap orders/settings", Some(&before), &after);

        assert!(diff.contains("--- ConfigMap orders/settings"));
        assert!(diff.contains("-  mode: slow"));
        assert!(diff.contains("+  mode: fast"));
        assert!(diff.contains("   size: '3'") || diff.contains("   size: \"3\""));
    }

    #[test]
    fn a_new_object_is_added_from_nothing() {
        let after = json!({ "kind": "ConfigMap" });

        let diff = unified("ConfigMap settings", None, &after);

        assert!(diff.starts_with("--- /dev/null"));
        assert!(diff.contains("+kind: ConfigMap"));
    }

    #[test]
    fn no_change_is_no_diff() {
        let same = json!({ "kind": "ConfigMap" });

        assert_eq!(unified("ConfigMap settings", Some(&same), &same), "");
    }

    #[test]
    fn secret_values_are_hidden_but_their_changes_still_show() {
        let mut before = json!({ "data": { "user": "YWRtaW4=", "password": "b2xk" } });
        let mut after = json!({ "data": { "user": "YWRtaW4=", "password": "bmV3" }, "stringData": { "token": "plain" } });

        hide_secret_values(Some(&mut before), &mut after);

        assert_eq!(
            before,
            json!({ "data": { "user": "***", "password": "*** (before)" } })
        );
        assert_eq!(
            after,
            json!({
                "data": { "user": "***", "password": "*** (after)" },
                "stringData": { "token": "*** (after)" }
            })
        );

        let diff = unified("Secret login", Some(&before), &after);
        assert!(!diff.contains("bmV3") && !diff.contains("b2xk") && !diff.contains("plain"));
    }
}
