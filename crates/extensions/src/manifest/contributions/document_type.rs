mod suffix;

use serde::{Deserialize, Serialize};

pub use suffix::Suffix;

use crate::manifest::{LocalId, OneLine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentType {
    #[serde(rename = "type")]
    kind: LocalId,
    title: OneLine,
    suffix: Suffix,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    panel: Option<LocalId>,
}

impl DocumentType {
    pub(crate) fn kind(&self) -> &LocalId {
        &self.kind
    }

    pub(crate) fn suffix(&self) -> &Suffix {
        &self.suffix
    }

    pub(crate) fn panel(&self) -> Option<&LocalId> {
        self.panel.as_ref()
    }

    pub(crate) fn kind_name(&self, extension: &str) -> String {
        format!("ext.{extension}.{}", self.kind)
    }
}
