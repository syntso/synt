use serde::Deserialize;

use super::Binding;

#[derive(Debug, Deserialize)]
pub(in crate::view) struct Submit {
    #[serde(rename = "title")]
    _title: String,

    #[serde(flatten)]
    binding: Binding,
}

impl Submit {
    pub(in crate::view) fn binding(&self) -> &Binding {
        &self.binding
    }
}
