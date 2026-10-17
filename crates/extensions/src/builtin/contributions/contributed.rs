use serde::Serialize;
use synt_proto::OneLine;

use crate::manifest::{Contributions, ExtensionId, Manifest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Contributed {
    extension: ExtensionId,
    name: OneLine,
    contributes: Contributions,
}

impl Contributed {
    pub(crate) fn of(manifest: &Manifest) -> Option<Self> {
        if manifest.contributes().is_empty() {
            return None;
        }

        Some(Self {
            extension: manifest.id().clone(),
            name: manifest.name().clone(),
            contributes: manifest.contributes().clone(),
        })
    }

    pub(crate) fn extension(&self) -> &ExtensionId {
        &self.extension
    }
}
