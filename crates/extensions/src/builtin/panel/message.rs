use std::sync::Arc;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::Extensions;
use crate::builtin::guard::refuse_extensions;
use crate::extension::on_guest_thread;
use crate::manifest::Panel;

#[derive(Clone)]
pub(crate) struct ExtensionPanelMessage {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MessageParams {
    pub extension: String,
    pub panel: String,
    pub message: Value,
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
            .map_or(Risk::Read, |loaded| loaded.ceiling)
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
        refuse_extensions(context)?;

        let loaded = self.extensions.loaded(&params.extension)?;
        let is_web = loaded
            .guest
            .manifest()
            .contributes
            .panel(&params.panel)
            .is_some_and(Panel::is_web);

        if !is_web {
            return Err(ActionError::InvalidParams(format!(
                "{} contributes no web panel {}",
                params.extension, params.panel
            )));
        }

        let guest = Arc::clone(&loaded.guest);
        let message = params.message.to_string();
        let context = context.clone();

        let reply = on_guest_thread(move |runtime| {
            guest.panel_message(&params.panel, &message, context, runtime)
        })
        .await?;

        Ok(serde_json::from_str(&reply).unwrap_or(Value::String(reply)))
    }
}
