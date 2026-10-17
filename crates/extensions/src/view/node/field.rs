use serde::Deserialize;

use super::Input;

#[derive(Debug, Deserialize)]
pub(in crate::view) struct Field {
    #[serde(rename = "name")]
    _name: String,

    #[serde(rename = "label")]
    _label: String,

    #[serde(rename = "input")]
    _input: Input,

    #[serde(default, rename = "options")]
    _options: Option<Vec<String>>,
}
