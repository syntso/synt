mod operation;

use serde::Deserialize;
use serde_json::Value;

use operation::Operation;

use crate::ExtensionError;

const MOST_OPERATIONS: usize = 1_000;

pub(crate) struct ViewPatch {
    operations: Vec<Operation>,
}

impl ViewPatch {
    pub(crate) fn parse(patch: &Value) -> Result<Self, ExtensionError> {
        let operations =
            Vec::<Operation>::deserialize(patch).map_err(ExtensionError::MalformedView)?;

        if operations.is_empty() || operations.len() > MOST_OPERATIONS {
            return Err(ExtensionError::PatchSize(MOST_OPERATIONS));
        }

        Ok(Self { operations })
    }

    pub(crate) fn check(&self, owns: &dyn Fn(&str) -> bool) -> Result<(), ExtensionError> {
        self.operations
            .iter()
            .try_for_each(|operation| operation.check(owns))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ViewPatch;

    fn owns(action: &str) -> bool {
        action.starts_with("uptime.")
    }

    #[test]
    fn accepts_replacing_values_and_nodes() {
        let patch = json!([
            { "op": "replace", "path": "/children/0/text", "value": "load 0.42" },
            { "op": "add", "path": "/children/-", "value": { "type": "text", "text": "new" } },
            { "op": "remove", "path": "/children/2" }
        ]);

        assert!(ViewPatch::parse(&patch).unwrap().check(&owns).is_ok());
    }

    #[test]
    fn refuses_other_operations_and_foreign_buttons() {
        for patch in [
            json!({ "op": "replace" }),
            json!([]),
            json!([{ "op": "move", "from": "/a", "path": "/b" }]),
            json!([{ "op": "replace", "path": "nope", "value": 1 }]),
            json!([{ "op": "add", "path": "/x" }]),
            json!([{ "op": "add", "path": "/x", "value": { "type": "button", "title": "Go", "action": "ssh.exec" } }]),
        ] {
            let checked = ViewPatch::parse(&patch).and_then(|parsed| parsed.check(&owns));

            assert!(checked.is_err(), "{patch}");
        }
    }
}
