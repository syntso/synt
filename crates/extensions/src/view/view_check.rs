use serde_json::{Map, Value};

const DEEPEST: usize = 32;
const MOST_NODES: usize = 10_000;
const MOST_LINES: usize = 5_000;

const DIRECTIONS: [&str; 2] = ["row", "column"];
const STYLES: [&str; 6] = ["title", "body", "muted", "mono", "error", "success"];
const INPUTS: [&str; 5] = ["text", "number", "toggle", "select", "textarea"];
const CHARTS: [&str; 2] = ["line", "bar"];

pub(crate) struct ViewCheck<'a> {
    owns: &'a dyn Fn(&str) -> bool,
    nodes: usize,
}

impl<'a> ViewCheck<'a> {
    pub(crate) fn new(owns: &'a dyn Fn(&str) -> bool) -> Self {
        Self { owns, nodes: 0 }
    }

    pub(crate) fn tree(mut self, tree: &Value) -> Result<(), String> {
        self.node(tree, 0)
    }

    pub(crate) fn value(mut self, value: &Value) -> Result<(), String> {
        self.anything(value, 0)
    }

    fn anything(&mut self, value: &Value, depth: usize) -> Result<(), String> {
        match value {
            Value::Object(fields) if fields.contains_key("type") => self.node(value, depth),
            Value::Object(fields) => fields
                .values()
                .try_for_each(|inner| self.anything(inner, depth + 1)),
            Value::Array(items) => items
                .iter()
                .try_for_each(|inner| self.anything(inner, depth + 1)),
            _ => Ok(()),
        }
    }

    fn node(&mut self, node: &Value, depth: usize) -> Result<(), String> {
        let fields = node.as_object().ok_or("a view node must be an object")?;
        let kind = text(fields, "type")?;

        self.nodes += 1;
        if self.nodes > MOST_NODES || depth > DEEPEST {
            return Err(format!(
                "a view holds at most {MOST_NODES} nodes, {DEEPEST} deep"
            ));
        }

        match kind {
            "stack" => self.stack(fields, depth),
            "text" => one_of(fields, "style", &STYLES, false).and(text(fields, "text").map(drop)),
            "log" => lines(fields),
            "list" => self.list(fields),
            "table" => table(fields),
            "keyvalue" => key_values(fields),
            "field" => field(fields),
            "form" => self.form(fields, depth),
            "button" => self.action(fields),
            "chart" => chart(fields),
            other => Err(format!("{other:?} is not a view node type")),
        }
    }

    fn stack(&mut self, fields: &Map<String, Value>, depth: usize) -> Result<(), String> {
        one_of(fields, "direction", &DIRECTIONS, false)?;

        array(fields, "children")?
            .iter()
            .try_for_each(|child| self.node(child, depth + 1))
    }

    fn list(&self, fields: &Map<String, Value>) -> Result<(), String> {
        array(fields, "items")?.iter().try_for_each(|item| {
            let item = item.as_object().ok_or("a list item must be an object")?;
            text(item, "title")?;

            match item.get("action") {
                Some(_) => self.action(item),
                None => Ok(()),
            }
        })
    }

    fn form(&mut self, fields: &Map<String, Value>, depth: usize) -> Result<(), String> {
        for child in array(fields, "children")? {
            let is_field = child.get("type").and_then(Value::as_str) == Some("field");
            if !is_field {
                return Err("a form holds only field nodes".to_owned());
            }

            self.node(child, depth + 1)?;
        }

        let submit = fields
            .get("submit")
            .and_then(Value::as_object)
            .ok_or("a form needs a submit with a title and an action")?;
        text(submit, "title")?;

        self.action(submit)
    }

    fn action(&self, fields: &Map<String, Value>) -> Result<(), String> {
        let action = text(fields, "action")?;

        if !(self.owns)(action) {
            return Err(format!(
                "a view may bind only the extension's own actions, not {action}"
            ));
        }

        if fields
            .get("params")
            .is_some_and(|params| !params.is_object())
        {
            return Err(format!("the params bound to {action} must be an object"));
        }

        match fields.get("target") {
            Some(target) if !target.is_string() => {
                Err(format!("the target bound to {action} must be a string"))
            }
            _ => Ok(()),
        }
    }
}

fn lines(fields: &Map<String, Value>) -> Result<(), String> {
    let lines = array(fields, "lines")?;

    if lines.len() > MOST_LINES || !lines.iter().all(Value::is_string) {
        return Err(format!("a log holds at most {MOST_LINES} lines of text"));
    }

    Ok(())
}

fn table(fields: &Map<String, Value>) -> Result<(), String> {
    let columns = array(fields, "columns")?;
    if !columns.iter().all(Value::is_string) {
        return Err("table columns must be text".to_owned());
    }

    array(fields, "rows")?.iter().try_for_each(|row| {
        let cells = row
            .as_array()
            .ok_or("a table row must be a list of cells")?;

        if cells.len() != columns.len() || !cells.iter().all(is_scalar) {
            return Err("every table row holds one plain value per column".to_owned());
        }

        Ok(())
    })
}

fn key_values(fields: &Map<String, Value>) -> Result<(), String> {
    array(fields, "items")?.iter().try_for_each(|item| {
        let item = item
            .as_object()
            .ok_or("a key-value item must be an object")?;
        text(item, "key")?;

        if item.get("value").is_some_and(is_scalar) {
            return Ok(());
        }

        Err("a key-value item needs a plain value".to_owned())
    })
}

fn field(fields: &Map<String, Value>) -> Result<(), String> {
    text(fields, "name")?;
    text(fields, "label")?;
    one_of(fields, "input", &INPUTS, true)?;

    match fields.get("options") {
        Some(options)
            if !options
                .as_array()
                .is_some_and(|options| options.iter().all(Value::is_string)) =>
        {
            Err("field options must be a list of text".to_owned())
        }
        _ => Ok(()),
    }
}

fn chart(fields: &Map<String, Value>) -> Result<(), String> {
    one_of(fields, "kind", &CHARTS, true)?;

    array(fields, "series")?.iter().try_for_each(|series| {
        let series = series
            .as_object()
            .ok_or("a chart series must be an object")?;
        text(series, "name")?;

        let points = array(series, "points")?;
        if points.iter().all(is_point) {
            return Ok(());
        }

        Err("chart points are [x, y] pairs of numbers".to_owned())
    })
}

fn text<'v>(fields: &'v Map<String, Value>, name: &str) -> Result<&'v str, String> {
    fields
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("a view node needs {name} as text"))
}

fn array<'v>(fields: &'v Map<String, Value>, name: &str) -> Result<&'v Vec<Value>, String> {
    fields
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("a view node needs {name} as a list"))
}

fn one_of(
    fields: &Map<String, Value>,
    name: &str,
    allowed: &[&str],
    required: bool,
) -> Result<(), String> {
    match fields.get(name) {
        None if !required => Ok(()),
        Some(Value::String(value)) if allowed.contains(&value.as_str()) => Ok(()),
        _ => Err(format!("{name} must be one of {}", allowed.join(", "))),
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn is_point(point: &Value) -> bool {
    point
        .as_array()
        .is_some_and(|pair| pair.len() == 2 && pair.iter().all(Value::is_number))
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::ViewCheck;

    fn checked(tree: &Value) -> Result<(), String> {
        let owns = |action: &str| action.starts_with("uptime.");

        ViewCheck::new(&owns).tree(tree)
    }

    #[test]
    fn accepts_every_node_type() {
        let tree = json!({
            "type": "stack",
            "direction": "column",
            "children": [
                { "type": "text", "text": "web-1", "style": "title" },
                { "type": "keyvalue", "items": [{ "key": "up", "value": "3 days" }] },
                { "type": "chart", "kind": "line", "series": [{ "name": "load", "points": [[0, 0.5], [1, 0.7]] }] },
                { "type": "table", "columns": ["pod", "ready"], "rows": [["api", true]] },
                { "type": "log", "lines": ["started"] },
                { "type": "list", "items": [{ "title": "web-1", "action": "uptime.check", "target": "web-1" }] },
                { "type": "form", "children": [{ "type": "field", "name": "every", "label": "Every", "input": "number" }],
                  "submit": { "title": "Save", "action": "uptime.save" } },
                { "type": "button", "title": "Refresh", "action": "uptime.check", "params": {} }
            ]
        });

        assert_eq!(checked(&tree), Ok(()));
    }

    #[test]
    fn buttons_bind_only_the_extensions_own_actions() {
        let tree = json!({ "type": "button", "title": "Wipe", "action": "ssh.exec", "params": { "command": "rm -rf /" } });

        let refusal = checked(&tree).unwrap_err();

        assert!(refusal.contains("own actions"), "{refusal}");
    }

    #[test]
    fn refuses_unknown_and_malformed_nodes() {
        for tree in [
            json!({ "type": "iframe", "src": "https://example.com" }),
            json!({ "type": "text" }),
            json!({ "type": "table", "columns": ["a"], "rows": [["x", "y"]] }),
            json!({ "type": "chart", "kind": "pie", "series": [] }),
            json!({ "type": "form", "children": [{ "type": "text", "text": "x" }], "submit": { "title": "Go", "action": "uptime.go" } }),
            json!("just text"),
        ] {
            assert!(checked(&tree).is_err(), "{tree}");
        }
    }

    #[test]
    fn refuses_trees_that_are_too_deep() {
        let mut tree = json!({ "type": "text", "text": "leaf" });
        for _ in 0..40 {
            tree = json!({ "type": "stack", "children": [tree] });
        }

        assert!(checked(&tree).is_err());
    }
}
