use std::collections::BTreeMap;

use super::DraftStep;
use super::literal::Literal;
use crate::runbooks::RunbookError;
use crate::runbooks::param::{ParamName, ParamSpec};

#[derive(Debug, Default)]
pub(super) struct Placeholders {
    names: BTreeMap<String, ParamName>,
    specs: Vec<ParamSpec>,
}

impl Placeholders {
    pub(super) fn for_steps(steps: &[DraftStep]) -> Result<Self, RunbookError> {
        let mut placeholders = Self::default();

        for literal in Literal::repeated_in(steps) {
            let name = placeholders.free_name(&literal.base_name())?;

            placeholders.specs.push(literal.spec(name.clone()));
            placeholders.names.insert(literal.text().to_owned(), name);
        }

        Ok(placeholders)
    }

    pub(super) fn lift(&self, text: &str) -> Option<String> {
        self.names.get(text).map(|name| format!("{{{name}}}"))
    }

    pub(super) fn into_specs(self) -> Vec<ParamSpec> {
        self.specs
    }

    fn free_name(&self, base: &str) -> Result<ParamName, RunbookError> {
        let taken = |name: &str| self.specs.iter().any(|spec| spec.name().as_str() == name);

        let mut name = base.to_owned();
        let mut counter = 2;

        while taken(&name) {
            name = format!("{base}_{counter}");
            counter += 1;
        }

        ParamName::new(name)
    }
}
