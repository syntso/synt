use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use super::ExtensionInfo;
use super::params::IdParams;
use crate::Extensions;
use crate::builtin::blocking::blocking;

#[derive(Clone)]
pub(crate) struct ExtensionRemove {
    extensions: Extensions,
}

impl ExtensionRemove {
    pub(crate) fn new(extensions: Extensions) -> Self {
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
        output.state_summary()
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
