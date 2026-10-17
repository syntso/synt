use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use super::info::PanelInfo;
use crate::Extensions;
use crate::builtin::CallingActor;

#[derive(Clone)]
pub(crate) struct ExtensionPanelOpen {
    extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct PanelParams {
    extension: String,
    panel: String,
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
        output.summary()
    }

    async fn run(&self, context: &Context, params: PanelParams) -> Result<PanelInfo, ActionError> {
        context.refuse_extensions("reach the views, panels or actions of an extension")?;

        let loaded = self.extensions.loaded(&params.extension)?;
        let contributes = loaded.guest().manifest().contributes();

        contributes
            .web_panel(&params.panel)
            .and_then(|panel| PanelInfo::of(params.extension.clone(), panel))
            .ok_or_else(|| {
                ActionError::InvalidParams(format!(
                    "{} contributes no web panel {}",
                    params.extension, params.panel
                ))
            })
    }
}
