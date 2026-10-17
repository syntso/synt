use serde::Serialize;
use serde_json::{Value, json};

use crate::extension::Loaded;
use crate::manifest::{Api, Capabilities, Description, ExtensionId, Manifest, OneLine};
use crate::sha256::Sha256;
use crate::shelf::Installed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ExtensionInfo {
    id: ExtensionId,
    name: Option<OneLine>,
    version: OneLine,
    api: Option<Api>,
    description: Option<Description>,
    signed: bool,
    enabled: bool,
    loaded: bool,
    sha256: Sha256,
    capabilities: Option<Capabilities>,
    actions: Vec<String>,
    problem: Option<String>,
}

impl ExtensionInfo {
    pub(crate) fn loaded(loaded: &Loaded) -> Self {
        let manifest = loaded.guest().manifest().clone();
        let resting = Self::resting(loaded.installed(), Some(manifest), None);

        Self {
            loaded: true,
            actions: loaded.action_ids(),
            ..resting
        }
    }

    pub(crate) fn resting(
        installed: &Installed,
        manifest: Option<Manifest>,
        problem: Option<String>,
    ) -> Self {
        Self {
            id: installed.id().clone(),
            name: manifest.as_ref().map(|manifest| manifest.name().clone()),
            version: installed.version().clone(),
            api: manifest.as_ref().map(Manifest::api),
            description: manifest
                .as_ref()
                .map(|manifest| manifest.description().clone()),
            signed: installed.signed(),
            enabled: installed.enabled(),
            loaded: false,
            sha256: installed.sha256().clone(),
            capabilities: manifest.map(|manifest| manifest.capabilities().clone()),
            actions: Vec::new(),
            problem,
        }
    }

    pub(crate) fn state_summary(&self) -> Value {
        json!({ "id": self.id, "version": self.version, "enabled": self.enabled })
    }

    pub(crate) fn install_summary(&self) -> Value {
        json!({
            "id": self.id,
            "version": self.version,
            "sha256": self.sha256,
            "signed": self.signed,
            "capabilities": self.capabilities,
            "actions": self.actions,
        })
    }
}
