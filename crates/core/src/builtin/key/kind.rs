use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KeyKind {
    #[default]
    Ed25519,
}
