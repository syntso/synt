use std::sync::Arc;

use serde_json::{Value, json};

use super::{ActionSpec, Guest};
use crate::package::Package;
use crate::shelf::Installed;

pub(crate) struct Candidate {
    pub(crate) package: Package,
    pub(crate) guest: Arc<Guest>,
    pub(crate) specs: Vec<ActionSpec>,
    pub(crate) replaces: Option<Installed>,
}

impl Candidate {
    pub(crate) fn shown(&self, source: &str) -> Value {
        let manifest = &self.package.manifest;
        let actions: Vec<&str> = self.specs.iter().map(|spec| spec.id.as_str()).collect();
        let replaces = self.replaces.as_ref().map(|current| {
            json!({
                "version": current.version,
                "signed": current.signed,
                "clears_storage": current.signed != self.package.signed,
            })
        });

        json!({
            "id": manifest.id,
            "name": manifest.name,
            "description": manifest.description,
            "repository": manifest.repository,
            "version": manifest.version,
            "from": source,
            "signed": self.package.signed,
            "sha256": self.package.sha256,
            "capabilities": manifest.capabilities,
            "actions": actions,
            "replaces": replaces,
        })
    }
}
