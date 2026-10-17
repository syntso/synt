mod variable;

use std::env;
use std::ffi::OsString;

use variable::Variable;

#[derive(Debug, Default)]
pub(super) struct Environment {
    variables: Vec<Variable>,
}

impl Environment {
    pub(super) fn set(&mut self, name: &str, value: impl Into<OsString>) {
        self.variables.push(Variable::new(name, value.into()));
    }

    pub(super) fn pass_on(&mut self, names: &[&str]) {
        for name in names {
            if let Some(value) = env::var_os(name) {
                self.set(name, value);
            }
        }
    }

    pub(super) fn variables(&self) -> &[Variable] {
        &self.variables
    }
}
