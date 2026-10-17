use std::sync::Arc;

use serde_json::{Value, json};
use synt_actions::Risk;

use super::{ActionSpec, Guest, Loaded};
use crate::install::Package;
use crate::shelf::Installed;

pub(crate) struct Candidate {
    package: Package,
    guest: Arc<Guest>,
    specs: Vec<ActionSpec>,
    ceiling: Risk,
    replaces: Option<Installed>,
}

impl Candidate {
    pub(crate) fn new(
        package: Package,
        guest: Arc<Guest>,
        specs: Vec<ActionSpec>,
        ceiling: Risk,
        replaces: Option<Installed>,
    ) -> Self {
        Self {
            package,
            guest,
            specs,
            ceiling,
            replaces,
        }
    }

    pub(crate) fn package(&self) -> &Package {
        &self.package
    }

    pub(crate) fn into_loaded(self, installed: Installed) -> Loaded {
        Loaded::new(self.guest, self.specs, self.ceiling, installed)
    }

    pub(crate) fn shown(&self, source: &str) -> Value {
        let manifest = self.package.manifest();
        let actions: Vec<&str> = self.specs.iter().map(|spec| spec.id().as_str()).collect();
        let replaces = self.replaces.as_ref().map(|current| {
            json!({
                "version": current.version(),
                "signed": current.signed(),
                "clears_storage": current.signed() != self.package.signed(),
            })
        });

        json!({
            "id": manifest.id(),
            "name": manifest.name(),
            "description": manifest.description(),
            "repository": manifest.repository(),
            "version": manifest.version(),
            "api": manifest.api(),
            "from": source,
            "signed": self.package.signed(),
            "sha256": self.package.sha256(),
            "capabilities": manifest.capabilities(),
            "contributes": manifest.contributes(),
            "actions": actions,
            "replaces": replaces,
        })
    }
}
