use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use super::ExtensionInfo;
use super::params::IdParams;
use crate::Extensions;
use crate::blocking::Blocking;

#[derive(Clone)]
pub(crate) struct ExtensionEnable {
    extensions: Extensions,
}

impl ExtensionEnable {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionEnable {
    const ID: &'static str = "extension.enable";
    const TITLE: &'static str = "Enable an installed extension";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = IdParams;
    type Output = ExtensionInfo;

    fn summary(&self, output: &ExtensionInfo) -> Value {
        output.state_summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: IdParams,
    ) -> Result<ExtensionInfo, ActionError> {
        let extensions = self.extensions.clone();

        Blocking::run(move || extensions.enable(&params.id)).await
    }
}
