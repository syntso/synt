use serde::Deserialize;
use serde_json::{Map, Value};

use crate::ExtensionError;
use crate::view::check::ViewCheck;

#[derive(Debug, Deserialize)]
pub(in crate::view) struct Binding {
    action: String,

    #[serde(default, rename = "params")]
    _params: Option<Map<String, Value>>,

    #[serde(default, rename = "target")]
    _target: Option<String>,
}

impl Binding {
    pub(in crate::view) fn check(&self, census: &ViewCheck<'_>) -> Result<(), ExtensionError> {
        census.bound(&self.action)
    }
}
