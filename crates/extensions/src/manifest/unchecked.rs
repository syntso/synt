use serde::Deserialize;

use super::{Api, Capabilities, Contributions, Description, ExtensionId, OneLine};

#[derive(Deserialize)]
pub(super) struct Unchecked {
    pub(super) id: ExtensionId,
    pub(super) name: OneLine,
    pub(super) version: OneLine,
    pub(super) api: Api,

    #[serde(default)]
    pub(super) description: Description,

    #[serde(default)]
    pub(super) repository: Option<String>,

    #[serde(default)]
    pub(super) capabilities: Capabilities,

    #[serde(default)]
    pub(super) contributes: Contributions,
}
