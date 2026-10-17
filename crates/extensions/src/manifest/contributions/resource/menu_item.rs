use serde::{Deserialize, Serialize};

use super::ResourceKind;
use crate::manifest::{ActionName, OneLine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    title: OneLine,
    action: ActionName,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    kinds: Vec<ResourceKind>,
}

impl MenuItem {
    pub(crate) fn action(&self) -> &ActionName {
        &self.action
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::MenuItem;

    fn item(kinds: &[&str]) -> Result<MenuItem, serde_json::Error> {
        serde_json::from_value(json!({
            "title": "Open",
            "action": "every.open",
            "kinds": kinds,
        }))
    }

    #[test]
    fn a_menu_entry_may_name_every_kind_of_resource() {
        assert!(item(&["host", "tunnel", "cluster", "database"]).is_ok());
        assert!(item(&[]).is_ok());
    }

    #[test]
    fn a_menu_entry_naming_an_unknown_kind_is_refused() {
        let error = item(&["printer"]).unwrap_err().to_string();

        assert!(error.contains("printer"), "{error}");
        assert!(error.contains("database"), "{error}");
    }
}
