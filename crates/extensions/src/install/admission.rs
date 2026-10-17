mod learned;

use std::sync::OnceLock;

use synt_actions::{Registry, Risk};

use learned::Learned;

use crate::ExtensionError;
use crate::manifest::{ExtensionId, Manifest};

const RESERVED: [&str; 2] = ["core", "extension"];

#[derive(Default)]
pub(crate) struct Admission {
    learned: OnceLock<Learned>,
}

impl Admission {
    pub(crate) fn learn(&self, registry: &Registry) {
        if self.learned.set(Learned::of(registry)).is_err() {
            eprintln!("synt-core: extensions: the core's actions were learned already");
        }
    }

    pub(crate) fn check_id(&self, id: &ExtensionId) -> Result<(), ExtensionError> {
        let id = id.as_str();
        let taken = RESERVED.contains(&id)
            || self
                .learned
                .get()
                .is_some_and(|learned| learned.takes_namespace(id));

        if taken {
            return Err(ExtensionError::IdTaken(id.to_owned()));
        }

        Ok(())
    }

    pub(crate) fn ceiling(&self, manifest: &Manifest) -> Result<Risk, ExtensionError> {
        let capabilities = manifest.capabilities();
        let calls = self.call_ceiling(manifest)?;

        Ok(calls.max(capabilities.ceiling()))
    }

    fn call_ceiling(&self, manifest: &Manifest) -> Result<Risk, ExtensionError> {
        manifest
            .capabilities()
            .calls()
            .try_fold(Risk::Read, |ceiling, action| {
                let risk = self
                    .learned
                    .get()
                    .and_then(|learned| learned.risk_of(action.as_str()))
                    .ok_or_else(|| ExtensionError::NotCallable(action.to_string()))?;

                Ok(ceiling.max(risk))
            })
    }
}
