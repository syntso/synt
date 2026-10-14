use serde::Serialize;

use crate::documents::DocumentInfo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ListedDocument {
    pub extension: String,

    #[serde(flatten)]
    pub info: DocumentInfo,
}

impl ListedDocument {
    pub(crate) fn new(extension: &str, info: DocumentInfo) -> Self {
        Self {
            extension: extension.to_owned(),
            info,
        }
    }
}
