use serde::Serialize;

use crate::documents::DocumentInfo;
use crate::manifest::ExtensionId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ListedDocument {
    extension: ExtensionId,

    #[serde(flatten)]
    info: DocumentInfo,
}

impl ListedDocument {
    pub(crate) fn new(extension: ExtensionId, info: DocumentInfo) -> Self {
        Self { extension, info }
    }
}
