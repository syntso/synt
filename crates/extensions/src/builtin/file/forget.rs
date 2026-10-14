use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::handle_params::HandleParams;
use crate::Extensions;
use crate::builtin::blocking::blocking;

#[derive(Clone)]
pub(crate) struct ExtensionFileForget {
    extensions: Extensions,
}

impl ExtensionFileForget {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionFileForget {
    const ID: &'static str = "extension.file.forget";
    const TITLE: &'static str = "Take back a picked file from an extension";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = HandleParams;
    type Output = Value;

    fn summary(&self, output: &Value) -> Value {
        output.clone()
    }

    async fn run(&self, _context: &Context, params: HandleParams) -> Result<Value, ActionError> {
        let extensions = self.extensions.clone();

        blocking(move || {
            let forgotten = extensions
                .services()
                .shelf
                .forget_pick(&params.extension, &params.handle)?;

            Ok(json!({
                "extension": params.extension,
                "handle": params.handle,
                "forgotten": forgotten,
            }))
        })
        .await
    }
}
