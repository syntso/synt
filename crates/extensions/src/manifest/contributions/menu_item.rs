use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::{check_text, refused};

const RESOURCE_KINDS: [&str; 4] = ["host", "tunnel", "cluster", "database"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    pub title: String,
    pub action: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<String>,
}

impl MenuItem {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_text("menu title", &self.title)?;

        match self
            .kinds
            .iter()
            .find(|kind| !RESOURCE_KINDS.contains(&kind.as_str()))
        {
            Some(kind) => Err(refused(format!(
                "menu {:?} names the resource kind {kind:?}; kinds are {}",
                self.title,
                RESOURCE_KINDS.join(", ")
            ))),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MenuItem;

    fn item(kinds: &[&str]) -> MenuItem {
        MenuItem {
            title: "Open".to_owned(),
            action: "every.open".to_owned(),
            kinds: kinds.iter().map(|kind| (*kind).to_owned()).collect(),
        }
    }

    #[test]
    fn a_menu_entry_may_name_every_kind_of_resource() {
        assert!(item(&["host", "tunnel", "cluster", "database"]).check().is_ok());
        assert!(item(&[]).check().is_ok());
    }

    #[test]
    fn a_menu_entry_naming_an_unknown_kind_is_refused() {
        let error = item(&["printer"]).check().unwrap_err().to_string();

        assert!(error.contains("printer"), "{error}");
        assert!(error.contains("database"), "{error}");
    }
}
