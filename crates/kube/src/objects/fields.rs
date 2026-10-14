use serde_json::Value;

const METADATA: &str = "metadata";

pub(crate) trait Fields {
    fn text(&self, field: &str) -> Option<&str>;

    fn metadata_text(&self, field: &str) -> Option<&str>;
}

impl Fields for Value {
    fn text(&self, field: &str) -> Option<&str> {
        self.get(field).and_then(Value::as_str)
    }

    fn metadata_text(&self, field: &str) -> Option<&str> {
        self.get(METADATA).and_then(|metadata| metadata.text(field))
    }
}
