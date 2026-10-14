use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use synt_actions::{Registry, Risk};

use crate::ExtensionError;
use crate::manifest::{FileAccess, Manifest};

const RESERVED: [&str; 2] = ["core", "extension"];
const OWN_NAMESPACE: &str = "extension.";

#[derive(Default)]
pub(crate) struct Admission {
    namespaces: OnceLock<BTreeSet<String>>,
    core_risks: OnceLock<BTreeMap<String, Risk>>,
}

impl Admission {
    pub(crate) fn learn(&self, registry: &Registry) {
        let _ = self.namespaces.set(namespaces(registry));
        let _ = self.core_risks.set(callable(registry));
    }

    pub(crate) fn check_id(&self, id: &str) -> Result<(), ExtensionError> {
        let taken = RESERVED.contains(&id)
            || self
                .namespaces
                .get()
                .is_some_and(|namespaces| namespaces.contains(id));

        if taken {
            return Err(ExtensionError::Refused(format!(
                "the id {id} is taken by the core's own actions"
            )));
        }

        Ok(())
    }

    pub(crate) fn ceiling(&self, manifest: &Manifest) -> Result<Risk, ExtensionError> {
        Ok(self.call_ceiling(manifest)?.max(reach_ceiling(manifest)))
    }

    fn call_ceiling(&self, manifest: &Manifest) -> Result<Risk, ExtensionError> {
        let callable = self.core_risks.get();

        manifest
            .capabilities
            .call
            .iter()
            .try_fold(Risk::Read, |ceiling, action| {
                let risk = callable
                    .and_then(|callable| callable.get(action))
                    .ok_or_else(|| {
                        ExtensionError::Refused(format!(
                            "capabilities.call names {action}, which is not a core action an \
                             extension may call"
                        ))
                    })?;

                Ok(ceiling.max(*risk))
            })
    }
}

fn reach_ceiling(manifest: &Manifest) -> Risk {
    let capabilities = &manifest.capabilities;
    let writes_files = capabilities.files == Some(FileAccess::ReadWrite);

    if capabilities.http.is_empty() && !capabilities.documents && !writes_files {
        return Risk::Read;
    }

    Risk::Write
}

fn namespaces(registry: &Registry) -> BTreeSet<String> {
    registry
        .ids()
        .filter_map(|id| id.as_str().split_once('.'))
        .map(|(namespace, _)| namespace.to_owned())
        .collect()
}

fn callable(registry: &Registry) -> BTreeMap<String, Risk> {
    registry
        .describe()
        .into_iter()
        .filter(|description| !description.person_only)
        .filter(|description| !description.id.as_str().starts_with(OWN_NAMESPACE))
        .map(|description| (description.id.as_str().to_owned(), description.risk))
        .collect()
}
