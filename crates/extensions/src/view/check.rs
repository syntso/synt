use serde_json::Value;

use super::node::Node;

const DEEPEST: usize = 32;
const MOST_NODES: usize = 10_000;

const DIRECTIONS: [&str; 2] = ["row", "column"];

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

    fn node(&mut self, value: &Value, depth: usize) -> Result<(), String> {
        let node = Node::of(value, "a view node")?;
        let kind = node.text("type")?;

        self.count(depth)?;

        match kind {
            "stack" => self.stack(node, depth),
            "text" => node.check_text(),
            "log" => node.check_log(),
            "list" => self.list(node),
            "table" => node.check_table(),
            "keyvalue" => node.check_key_values(),
            "field" => node.check_field(),
            "form" => self.form(node, depth),
            "button" => self.action(node),
            "chart" => node.check_chart(),
            other => Err(format!("{other:?} is not a view node type")),
        }
    }

    fn count(&mut self, depth: usize) -> Result<(), String> {
        self.nodes += 1;

        if self.nodes > MOST_NODES || depth > DEEPEST {
            return Err(format!(
                "a view holds at most {MOST_NODES} nodes, {DEEPEST} deep"
            ));
        }

        Ok(())
    }

    fn stack(&mut self, node: Node<'_>, depth: usize) -> Result<(), String> {
        node.one_of("direction", &DIRECTIONS, false)?;

        node.list("children")?
            .iter()
            .try_for_each(|child| self.node(child, depth + 1))
    }

    fn list(&self, node: Node<'_>) -> Result<(), String> {
        node.list("items")?.iter().try_for_each(|item| {
            let item = Node::of(item, "a list item")?;
            item.text("title")?;

            match item.get("action") {
                Some(_) => self.action(item),
                None => Ok(()),
            }
        })
    }

    fn form(&mut self, node: Node<'_>, depth: usize) -> Result<(), String> {
        for child in node.list("children")? {
            let is_field = child.get("type").and_then(Value::as_str) == Some("field");
            if !is_field {
                return Err("a form holds only field nodes".to_owned());
            }

            self.node(child, depth + 1)?;
        }

        let submit = node
            .get("submit")
            .filter(|submit| submit.is_object())
            .ok_or("a form needs a submit with a title and an action")?;
        let submit = Node::of(submit, "a form submit")?;
        submit.text("title")?;

        self.action(submit)
    }

    fn action(&self, node: Node<'_>) -> Result<(), String> {
        let action = node.text("action")?;

        if !(self.owns)(action) {
            return Err(format!(
                "a view may bind only the extension's own actions, not {action}"
            ));
        }

        if node.get("params").is_some_and(|params| !params.is_object()) {
            return Err(format!("the params bound to {action} must be an object"));
        }

        match node.get("target") {
            Some(target) if !target.is_string() => {
                Err(format!("the target bound to {action} must be a string"))
            }
            _ => Ok(()),
        }
    }
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
