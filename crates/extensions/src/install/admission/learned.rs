use std::collections::{BTreeMap, BTreeSet};

use synt_actions::{Registry, Risk};

const OWN_NAMESPACE: &str = "extension.";

pub(super) struct Learned {
    namespaces: BTreeSet<String>,
    core_risks: BTreeMap<String, Risk>,
}

impl Learned {
    pub(super) fn of(registry: &Registry) -> Self {
        Self {
            namespaces: Self::namespaces(registry),
            core_risks: Self::callable(registry),
        }
    }

    pub(super) fn takes_namespace(&self, id: &str) -> bool {
        self.namespaces.contains(id)
    }

    pub(super) fn risk_of(&self, action: &str) -> Option<Risk> {
        self.core_risks.get(action).copied()
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
}
