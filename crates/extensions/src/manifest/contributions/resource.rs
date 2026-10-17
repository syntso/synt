mod kind;
mod menu_item;

use serde::{Deserialize, Serialize};

pub use kind::ResourceKind;
pub use menu_item::MenuItem;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceContributions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    menu: Vec<MenuItem>,
}

impl ResourceContributions {
    pub(crate) fn is_empty(&self) -> bool {
        self.menu.is_empty()
    }

    pub(crate) fn menu(&self) -> &[MenuItem] {
        &self.menu
    }
}
