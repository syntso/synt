use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};

use super::ExtensionInfo;
use crate::Extensions;

#[derive(Clone)]
pub(crate) struct ExtensionList {
    extensions: Extensions,
}

impl ExtensionList {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionList {
    const ID: &'static str = "extension.list";
    const TITLE: &'static str = "List the installed extensions";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<ExtensionInfo>;

    fn summary(&self, output: &Vec<ExtensionInfo>) -> Value {
        json!({ "extensions": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<ExtensionInfo>, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || extensions.list()).await
    }
}
