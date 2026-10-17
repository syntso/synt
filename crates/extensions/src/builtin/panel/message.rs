use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::Extensions;
use crate::blocking::Blocking;
use crate::builtin::CallingActor;

#[derive(Clone)]
pub(crate) struct ExtensionPanelMessage {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MessageParams {
    extension: String,
    panel: String,
    message: Value,
}

impl ExtensionPanelMessage {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionPanelMessage {
    const ID: &'static str = "extension.panel.message";
    const TITLE: &'static str = "Pass a message from a web panel to its extension";
    const RISK: Risk = Risk::Read;

    type Params = MessageParams;
    type Output = Value;

    fn risk(&self, _target: Option<&ResourceRef>, params: &MessageParams) -> Risk {
        self.extensions
            .loaded(&params.extension)
            .map_or(Risk::Read, |loaded| loaded.ceiling())
    }

    fn recorded_params(&self, params: &Value) -> Value {
        json!({
            "extension": params.get("extension"),
            "panel": params.get("panel"),
            "message_bytes": params.get("message").map(|message| message.to_string().len()),
        })
    }

    fn summary(&self, output: &Value) -> Value {
        json!({ "reply_bytes": output.to_string().len() })
    }

    async fn run(&self, context: &Context, params: MessageParams) -> Result<Value, ActionError> {
        context.refuse_extensions("reach the views, panels or actions of an extension")?;

        let loaded = self.extensions.loaded(&params.extension)?;
        let guest = Arc::clone(loaded.guest());

        if guest
            .manifest()
            .contributes()
            .web_panel(&params.panel)
            .is_none()
        {
            return Err(ActionError::InvalidParams(format!(
                "{} contributes no web panel {}",
                params.extension, params.panel
            )));
        }

        let message = params.message.to_string();
        let context = context.clone();

        let reply = Blocking::run_with_runtime(move |runtime| {
            guest.panel_message(&params.panel, &message, context, runtime)
        })
        .await?;

        Ok(reply.into_value())
    }
}
