use std::path::Path;

use serde::Serialize;

use crate::manifest::FileAccess;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Picked {
    pub(crate) handle: String,
    pub(crate) name: String,
    pub(crate) access: FileAccess,

    #[serde(skip)]
    pub(crate) path: String,
}

impl Picked {
    pub(crate) fn name_of(path: &str) -> String {
        Path::new(path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
