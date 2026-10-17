use serde::Deserialize;
use serde_json::Value;

use crate::error::ViewError;
use crate::view::ViewCheck;

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub(super) enum Operation {
    Add { path: String, value: Value },
    Remove { path: String },
    Replace { path: String, value: Value },
}

impl Operation {
    pub(super) fn check(&self, owns: &dyn Fn(&str) -> bool) -> Result<(), ViewError> {
        let (path, value) = match self {
            Self::Add { path, value } | Self::Replace { path, value } => (path, Some(value)),
            Self::Remove { path } => (path, None),
        };

        if !path.is_empty() && !path.starts_with('/') {
            return Err(ViewError::PatchPath);
        }

        match value {
            Some(value) => ViewCheck::new(owns).value(value),
            None => Ok(()),
        }
    }
}
