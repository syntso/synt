use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::Extensions;
use crate::blocking::Blocking;

#[derive(Clone)]
pub(crate) struct ExtensionFileForget {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HandleParams {
    pub extension: String,
    pub handle: String,
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

        Blocking::run(move || {
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
