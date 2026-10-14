use serde::Serialize;

use crate::documents::DocumentInfo;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ListedDocument {
    pub extension: String,

    #[serde(flatten)]
    pub info: DocumentInfo,
}
