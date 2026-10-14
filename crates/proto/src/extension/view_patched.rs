use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Notice;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewPatched {
    pub extension: String,
    pub view: String,
    pub patch: Value,
}

impl Notice for ViewPatched {
    const METHOD: &'static str = "extension.view.patched";
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ViewPatched;
    use crate::Notice;

    #[test]
    fn a_patch_travels_as_a_notification() {
        let patched = ViewPatched {
            extension: "uptime".to_owned(),
            view: "load".to_owned(),
            patch: json!([{ "op": "replace", "path": "/text", "value": "0.42" }]),
        };

        let notification = patched.to_notification().unwrap();

        assert_eq!(notification.method, "extension.view.patched");
        assert_eq!(notification.params::<ViewPatched>().unwrap(), patched);
    }
}
