use serde::Deserialize;
use serde_json::{Map, Value};

use crate::ExtensionError;
use crate::view::check::ViewCheck;

#[derive(Debug, Deserialize)]
pub(in crate::view) struct ListItem {
    #[serde(rename = "title")]
    _title: String,

    #[serde(default)]
    action: Option<String>,

    #[serde(default, rename = "params")]
    _params: Option<Map<String, Value>>,

    #[serde(default, rename = "target")]
    _target: Option<String>,
}

impl ListItem {
    pub(in crate::view) fn check(&self, census: &ViewCheck<'_>) -> Result<(), ExtensionError> {
        match &self.action {
            Some(action) => census.bound(action),
            None => Ok(()),
        }
    }
}
