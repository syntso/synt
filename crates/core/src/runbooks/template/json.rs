use serde_json::{Map, Value};

use super::TextTemplate;
use crate::runbooks::param::Values;

pub(crate) trait JsonTemplate: Sized {
    fn render(&self, values: &Values) -> Result<Self, String>;

    fn placeholders(&self) -> Vec<String>;

    fn escaped(&self) -> Self;
}

impl JsonTemplate for Value {
    fn render(&self, values: &Values) -> Result<Self, String> {
        match self {
            Value::String(text) => TextTemplate::parse(text).render(values),
            Value::Array(items) => items
                .iter()
                .map(|item| item.render(values))
                .collect::<Result<_, _>>()
                .map(Value::Array),
            Value::Object(fields) => fields.render(values).map(Value::Object),
            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(self.clone()),
        }
    }

    fn placeholders(&self) -> Vec<String> {
        match self {
            Value::String(text) => TextTemplate::parse(text)
                .params()
                .map(str::to_owned)
                .collect(),
            Value::Array(items) => items.iter().flat_map(Self::placeholders).collect(),
            Value::Object(fields) => fields.placeholders(),
            Value::Null | Value::Bool(_) | Value::Number(_) => Vec::new(),
        }
    }

    fn escaped(&self) -> Self {
        match self {
            Value::String(text) => Value::String(TextTemplate::escape(text)),
            Value::Array(items) => Value::Array(items.iter().map(Self::escaped).collect()),
            Value::Object(fields) => Value::Object(fields.escaped()),
            Value::Null | Value::Bool(_) | Value::Number(_) => self.clone(),
        }
    }
}

impl JsonTemplate for Map<String, Value> {
    fn render(&self, values: &Values) -> Result<Self, String> {
        self.iter()
            .map(|(key, value)| Ok((key.clone(), value.render(values)?)))
            .collect()
    }

    fn placeholders(&self) -> Vec<String> {
        self.values().flat_map(Value::placeholders).collect()
    }

    fn escaped(&self) -> Self {
        self.iter()
            .map(|(key, value)| (key.clone(), value.escaped()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::JsonTemplate;
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

        let rendered = params.render(&values).unwrap();

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

        let mut names = params.placeholders();
        names.sort();

        assert_eq!(names, ["x", "x", "y", "z"]);
    }

    #[test]
    fn escaped_params_render_to_the_original() {
        let params = json!({ "command": "awk '{print}'", "list": ["{a}"] });

        let rendered = params.escaped().render(&Values::default()).unwrap();

        assert_eq!(rendered, params);
    }
}
