use serde::{Deserialize, Serialize};

use crate::ExtensionError;
use crate::manifest::checks::{check_local_id, check_text, refused};

const LONGEST_SUFFIX: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentType {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub suffix: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel: Option<String>,
}

impl DocumentType {
    pub(crate) fn check(&self) -> Result<(), ExtensionError> {
        check_local_id("document type", &self.kind)?;
        check_text("document title", &self.title)?;

        if let Some(panel) = &self.panel {
            check_local_id("document panel", panel)?;
        }

        let plain = self
            .suffix
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit());

        if self.suffix.is_empty() || self.suffix.len() > LONGEST_SUFFIX || !plain {
            return Err(refused(format!(
                "document suffix {:?} must be 1 to {LONGEST_SUFFIX} lowercase letters and digits",
                self.suffix
            )));
        }

        Ok(())
    }

    pub fn kind_name(&self, extension: &str) -> String {
        format!("ext.{extension}.{}", self.kind)
    }
}
