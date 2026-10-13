use super::Sandbox;
use crate::bindings::ExtensionPre;

#[derive(Clone)]
pub(crate) struct Compiled {
    pre: ExtensionPre<Sandbox>,
}

impl Compiled {
    pub(super) fn new(pre: ExtensionPre<Sandbox>) -> Self {
        Self { pre }
    }

    pub(super) fn pre(&self) -> &ExtensionPre<Sandbox> {
        &self.pre
    }
}
