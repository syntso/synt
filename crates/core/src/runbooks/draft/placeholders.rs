use std::collections::BTreeMap;

use super::literal::Literal;
use crate::runbooks::param::ParamSpec;

#[derive(Debug, Default)]
pub(super) struct Placeholders {
    names: BTreeMap<String, String>,
}

impl Placeholders {
    pub(super) fn naming(literals: &[Literal]) -> Self {
        let mut placeholders = Self::default();
        let mut taken: Vec<String> = Vec::new();

        for literal in literals {
            let base = literal.base_name();
            let mut name = base.clone();
            let mut counter = 2;

            while taken.contains(&name) {
                name = format!("{base}_{counter}");
                counter += 1;
            }

            taken.push(name.clone());
            placeholders.names.insert(literal.text.clone(), name);
        }

        placeholders
    }

    pub(super) fn specs(&self, literals: &[Literal]) -> Vec<ParamSpec> {
        literals
            .iter()
            .filter_map(|literal| self.names.get(&literal.text).map(|name| literal.spec(name)))
            .collect()
    }

    pub(super) fn lift(&self, text: &str) -> Option<String> {
        self.names.get(text).map(|name| format!("{{{name}}}"))
    }
}
