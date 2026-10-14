use serde_json::{Map, Value};

use super::Template;
use crate::runbooks::param::Values;

pub(crate) fn render_json(value: &Value, values: &Values) -> Result<Value, String> {
    match value {
        Value::String(text) => Template::parse(text).render(values),
        Value::Array(items) => items
            .iter()
            .map(|item| render_json(item, values))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(fields) => render_fields(fields, values).map(Value::Object),
        Value::Null | Value::Bool(_) | Value::Number(_) => Ok(value.clone()),
    }
}

pub(crate) fn render_fields(
    fields: &Map<String, Value>,
    values: &Values,
) -> Result<Map<String, Value>, String> {
    fields
        .iter()
        .map(|(key, value)| Ok((key.clone(), render_json(value, values)?)))
        .collect()
}

pub(crate) fn params_in(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => Template::parse(text).params().map(str::to_owned).collect(),
        Value::Array(items) => items.iter().flat_map(params_in).collect(),
        Value::Object(fields) => fields.values().flat_map(params_in).collect(),
        Value::Null | Value::Bool(_) | Value::Number(_) => Vec::new(),
    }
}

pub(crate) fn escape_json(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(Template::escape(text)),
        Value::Array(items) => Value::Array(items.iter().map(escape_json).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), escape_json(value)))
                .collect(),
        ),
        Value::Null | Value::Bool(_) | Value::Number(_) => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{escape_json, params_in, render_json};
    use crate::runbooks::param::Values;

    #[test]
    fn every_string_inside_params_is_rendered() {
        let values = Values::from_pairs([("path", json!("/etc/app.conf")), ("bytes", json!(64))]);
        let params = json!({
            "path": "{path}",
            "length": "{bytes}",
            "flags": ["--from={path}", 3, null],
            "nested": { "create": true },
        });

        let rendered = render_json(&params, &values).unwrap();

        assert_eq!(
            rendered,
            json!({
                "path": "/etc/app.conf",
                "length": 64,
                "flags": ["--from=/etc/app.conf", 3, null],
                "nested": { "create": true },
            })
        );
    }

    #[test]
    fn the_params_used_anywhere_are_found() {
        let params = json!({ "a": "{x}", "b": ["{y} {x}"], "c": { "d": "{z}" }, "e": 1 });

        let mut names = params_in(&params);
        names.sort();

        assert_eq!(names, ["x", "x", "y", "z"]);
    }

    #[test]
    fn escaped_params_render_to_the_original() {
        let params = json!({ "command": "awk '{print}'", "list": ["{a}"] });

        let rendered = render_json(&escape_json(&params), &Values::default()).unwrap();

        assert_eq!(rendered, params);
    }
}
