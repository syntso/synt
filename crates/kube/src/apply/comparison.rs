use serde_json::{Map, Value};
use similar::TextDiff;

use crate::Change;

const CONTEXT_LINES: usize = 3;
const NOTHING: &str = "/dev/null";
const HIDDEN: &str = "***";
const HIDDEN_BEFORE: &str = "*** (before)";
const HIDDEN_AFTER: &str = "*** (after)";
const SECRET_FIELDS: [&str; 2] = ["data", "stringData"];

pub(crate) struct Comparison {
    before: Option<Value>,
    after: Value,
}

impl Comparison {
    pub(crate) fn new(before: Option<Value>, after: Value) -> Self {
        Self { before, after }
    }

    pub(crate) fn change(&self) -> Change {
        match &self.before {
            None => Change::Create,
            Some(before) if *before == self.after => Change::Unchanged,
            Some(_) => Change::Update,
        }
    }

    pub(crate) fn hide_secret_values(&mut self) {
        for field in SECRET_FIELDS {
            let old = self
                .before
                .as_mut()
                .and_then(|before| take_map(before, field));
            let new = take_map(&mut self.after, field);

            if let (Some(before), Some(old)) = (self.before.as_mut(), &old) {
                put_map(before, field, hidden(old, new.as_ref(), HIDDEN_BEFORE));
            }
            if let Some(new) = &new {
                put_map(
                    &mut self.after,
                    field,
                    hidden(new, old.as_ref(), HIDDEN_AFTER),
                );
            }
        }
    }

    pub(crate) fn unified(&self, label: &str) -> String {
        let before_text = self.before.as_ref().map(yaml).unwrap_or_default();
        let after_text = yaml(&self.after);

        if before_text == after_text {
            return String::new();
        }

        let old_label = if self.before.is_some() {
            label
        } else {
            NOTHING
        };

        TextDiff::from_lines(&before_text, &after_text)
            .unified_diff()
            .context_radius(CONTEXT_LINES)
            .header(old_label, label)
            .to_string()
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

    use super::Comparison;
    use crate::Change;

    #[test]
    fn a_change_shows_as_a_unified_diff_of_yaml() {
        let before = json!({ "kind": "ConfigMap", "data": { "mode": "slow", "size": "3" } });
        let after = json!({ "kind": "ConfigMap", "data": { "mode": "fast", "size": "3" } });

        let comparison = Comparison::new(Some(before), after);
        let diff = comparison.unified("ConfigMap orders/settings");

        assert_eq!(comparison.change(), Change::Update);
        assert!(diff.contains("--- ConfigMap orders/settings"));
        assert!(diff.contains("-  mode: slow"));
        assert!(diff.contains("+  mode: fast"));
        assert!(diff.contains("   size: '3'") || diff.contains("   size: \"3\""));
    }

    #[test]
    fn a_new_object_is_added_from_nothing() {
        let comparison = Comparison::new(None, json!({ "kind": "ConfigMap" }));
        let diff = comparison.unified("ConfigMap settings");

        assert_eq!(comparison.change(), Change::Create);
        assert!(diff.starts_with("--- /dev/null"));
        assert!(diff.contains("+kind: ConfigMap"));
    }

    #[test]
    fn no_change_is_no_diff() {
        let same = json!({ "kind": "ConfigMap" });
        let comparison = Comparison::new(Some(same.clone()), same);

        assert_eq!(comparison.change(), Change::Unchanged);
        assert_eq!(comparison.unified("ConfigMap settings"), "");
    }

    #[test]
    fn secret_values_are_hidden_but_their_changes_still_show() {
        let before = json!({ "data": { "user": "YWRtaW4=", "password": "b2xk" } });
        let after = json!({ "data": { "user": "YWRtaW4=", "password": "bmV3" }, "stringData": { "token": "plain" } });

        let mut comparison = Comparison::new(Some(before), after);
        comparison.hide_secret_values();

        assert_eq!(
            comparison.before,
            Some(json!({ "data": { "user": "***", "password": "*** (before)" } }))
        );
        assert_eq!(
            comparison.after,
            json!({
                "data": { "user": "***", "password": "*** (after)" },
                "stringData": { "token": "*** (after)" }
            })
        );

        let diff = comparison.unified("Secret login");
        assert!(!diff.contains("bmV3") && !diff.contains("b2xk") && !diff.contains("plain"));
    }

    #[test]
    fn a_new_secret_is_hidden_too() {
        let mut comparison = Comparison::new(None, json!({ "data": { "token": "c2VjcmV0" } }));
        comparison.hide_secret_values();

        assert_eq!(
            comparison.after,
            json!({ "data": { "token": "*** (after)" } })
        );
    }
}
