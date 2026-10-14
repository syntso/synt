use serde::{Deserialize, Serialize};

use super::MenuItem;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceContributions {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub menu: Vec<MenuItem>,
}

impl ResourceContributions {
    pub fn is_empty(&self) -> bool {
        self.menu.is_empty()
    }
}
