use data_encoding::BASE64;
use serde::Serialize;
use serde_json::{Value, json};

use crate::manifest::AssetPath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AssetFile {
    path: AssetPath,
    mime: &'static str,
    bytes: usize,
    data: String,
}

impl AssetFile {
    pub(crate) fn new(path: AssetPath, mime: &'static str, data: &[u8]) -> Self {
        Self {
            path,
            mime,
            bytes: data.len(),
            data: BASE64.encode(data),
        }
    }

    pub(crate) fn summary(&self) -> Value {
        json!({ "path": self.path, "mime": self.mime, "bytes": self.bytes })
    }
}
