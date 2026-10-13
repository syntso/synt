use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::blocking::blocking;
use super::{ExtensionInfo, IdParams};
use crate::Extensions;

#[derive(Clone)]
pub struct ExtensionEnable {
    extensions: Extensions,
}

impl ExtensionEnable {
    pub fn new(extensions: Extensions) -> Self {
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
        json!({ "id": output.id, "version": output.version, "enabled": output.enabled })
    }

    async fn run(
        &self,
        _context: &Context,
        params: IdParams,
    ) -> Result<ExtensionInfo, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || extensions.enable(&params.id)).await
    }
}
