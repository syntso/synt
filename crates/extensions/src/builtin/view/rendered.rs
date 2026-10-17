use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct Rendered {
    extension: String,
    view: String,
    tree: Value,
}

impl Rendered {
    pub(crate) fn new(extension: String, view: String, tree: Value) -> Self {
        Self {
            extension,
            view,
            tree,
        }
    }

    pub(crate) fn summary(&self) -> Value {
        json!({
            "extension": self.extension,
            "view": self.view,
            "bytes": self.tree.to_string().len(),
        })
    }
}
