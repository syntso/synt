use serde::{Deserialize, Serialize};

use crate::manifest::{AssetPath, LocalId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum PanelKind {
    View(LocalId),
    Entry(AssetPath),
}
