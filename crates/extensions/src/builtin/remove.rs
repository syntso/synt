use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::blocking::blocking;
use super::{ExtensionInfo, IdParams};
use crate::Extensions;

#[derive(Clone)]
pub struct ExtensionRemove {
    extensions: Extensions,
}

impl ExtensionRemove {
    pub fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionRemove {
    const ID: &'static str = "extension.remove";
    const TITLE: &'static str = "Remove an extension with its files and storage";
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

        blocking(move || extensions.remove(&params.id)).await
    }
}
