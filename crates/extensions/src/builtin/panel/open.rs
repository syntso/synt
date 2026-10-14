use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::info::PanelInfo;
use crate::Extensions;
use crate::builtin::guard::refuse_extensions;

#[derive(Clone)]
pub(crate) struct ExtensionPanelOpen {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PanelParams {
    pub extension: String,
    pub panel: String,
}

impl ExtensionPanelOpen {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self { extensions }
    }
}

impl Action for ExtensionPanelOpen {
    const ID: &'static str = "extension.panel.open";
    const TITLE: &'static str = "Describe a web panel of an extension for the app to show";
    const RISK: Risk = Risk::Read;

    type Params = PanelParams;
    type Output = PanelInfo;

    fn summary(&self, output: &PanelInfo) -> Value {
        json!({ "extension": output.extension, "panel": output.panel })
    }

    async fn run(&self, context: &Context, params: PanelParams) -> Result<PanelInfo, ActionError> {
        refuse_extensions(context)?;

        let loaded = self.extensions.loaded(&params.extension)?;
        let web_panel = loaded
            .guest
            .manifest()
            .contributes
            .panel(&params.panel)
            .and_then(|panel| Some((panel.title.clone(), panel.entry.clone()?)));

        let Some((title, entry)) = web_panel else {
            return Err(ActionError::InvalidParams(format!(
                "{} contributes no web panel {}",
                params.extension, params.panel
            )));
        };

        Ok(PanelInfo::new(params.extension, params.panel, title, entry))
    }
}
