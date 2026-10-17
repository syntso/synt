use serde_json::Value;

use super::node::ViewNode;
use crate::error::ViewError;

const DEEPEST: usize = 32;
const MOST_NODES: usize = 10_000;

pub(crate) struct ViewCheck<'a> {
    owns: &'a dyn Fn(&str) -> bool,
    nodes: usize,
}

impl<'a> ViewCheck<'a> {
    pub(crate) fn new(owns: &'a dyn Fn(&str) -> bool) -> Self {
        Self { owns, nodes: 0 }
    }

    pub(crate) fn tree(mut self, tree: &Value) -> Result<(), ViewError> {
        self.node(tree, 0)
    }

    pub(crate) fn value(mut self, value: &Value) -> Result<(), ViewError> {
        self.anything(value, 0)
    }

    pub(super) fn count(&mut self, depth: usize) -> Result<(), ViewError> {
        self.nodes += 1;

        if self.nodes > MOST_NODES || depth > DEEPEST {
            return Err(ViewError::TooLarge {
                most: MOST_NODES,
                deepest: DEEPEST,
            });
        }

        Ok(())
    }

    pub(super) fn bound(&self, action: &str) -> Result<(), ViewError> {
        if !(self.owns)(action) {
            return Err(ViewError::ForeignAction(action.to_owned()));
        }

        Ok(())
    }

    fn node(&mut self, value: &Value, depth: usize) -> Result<(), ViewError> {
        ViewNode::parse(value)?.check(self, depth)
    }

    fn anything(&mut self, value: &Value, depth: usize) -> Result<(), ViewError> {
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
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::ViewCheck;
    use crate::error::ViewError;

    fn checked(tree: &Value) -> Result<(), ViewError> {
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

        assert!(checked(&tree).is_ok());
    }

    #[test]
    fn buttons_bind_only_the_extensions_own_actions() {
        let tree = json!({ "type": "button", "title": "Wipe", "action": "ssh.exec", "params": { "command": "rm -rf /" } });

        let refusal = checked(&tree).unwrap_err().to_string();

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
