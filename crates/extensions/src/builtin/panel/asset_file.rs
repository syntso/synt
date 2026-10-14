use data_encoding::BASE64;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AssetFile {
    pub path: String,
    pub mime: &'static str,
    pub bytes: usize,
    pub data: String,
}

impl AssetFile {
    pub(crate) fn new(path: String, mime: &'static str, data: &[u8]) -> Self {
        Self {
            path,
            mime,
            bytes: data.len(),
            data: BASE64.encode(data),
        }
    }
}
