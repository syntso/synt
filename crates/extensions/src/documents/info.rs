use serde::Serialize;

use crate::sha256::Sha256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DocumentInfo {
    pub(crate) project: String,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) revision: Sha256,
    pub(crate) size: u64,
}
