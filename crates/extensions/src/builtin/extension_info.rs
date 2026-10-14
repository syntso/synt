use serde::Serialize;
use serde_json::{Value, json};

use crate::extension::Loaded;
use crate::manifest::{Capabilities, Manifest};
use crate::shelf::Installed;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ExtensionInfo {
    pub id: String,
    pub name: Option<String>,
    pub version: String,
    pub description: Option<String>,
    pub signed: bool,
    pub enabled: bool,
    pub loaded: bool,
    pub sha256: String,
    pub capabilities: Option<Capabilities>,
    pub actions: Vec<String>,
    pub problem: Option<String>,
}

impl ExtensionInfo {
    pub(crate) fn loaded(loaded: &Loaded) -> Self {
        let manifest = loaded.guest.manifest();

        Self {
            loaded: true,
            actions: loaded.action_ids(),
            ..Self::resting(loaded.installed.clone(), Some(manifest.clone()), None)
        }
    }

    pub(crate) fn resting(
        installed: Installed,
        manifest: Option<Manifest>,
        problem: Option<String>,
    ) -> Self {
        Self {
            id: installed.id,
            name: manifest.as_ref().map(|manifest| manifest.name.clone()),
            version: installed.version,
            description: manifest
                .as_ref()
                .map(|manifest| manifest.description.clone()),
            signed: installed.signed,
            enabled: installed.enabled,
            loaded: false,
            sha256: installed.sha256,
            capabilities: manifest.map(|manifest| manifest.capabilities),
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
