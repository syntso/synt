use std::ffi::{OsStr, OsString};

#[derive(Debug)]
pub(in crate::detached) struct Variable {
    name: OsString,
    value: OsString,
}

impl Variable {
    pub(super) fn new(name: &str, value: OsString) -> Self {
        Self {
            name: OsString::from(name),
            value,
        }
    }

    pub(in crate::detached) fn name(&self) -> &OsStr {
        &self.name
    }

    pub(in crate::detached) fn value(&self) -> &OsStr {
        &self.value
    }
}
