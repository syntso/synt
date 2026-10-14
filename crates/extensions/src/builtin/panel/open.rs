use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::panel_info::PanelInfo;
use super::panel_params::PanelParams;
use crate::Extensions;
use crate::builtin::guard::refuse_extensions;

pub(crate) const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' \
     'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'none'; \
     frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; \
     frame-ancestors 'self'";

#[derive(Clone)]
pub(crate) struct ExtensionPanelOpen {
    extensions: Extensions,
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

        let loaded = self
            .extensions
            .loaded(&params.extension)
            .map_err(|error| ActionError::failed(error.to_string()))?;
        let manifest = loaded.guest.manifest();

        let (title, entry) = manifest
            .contributes
            .panel(&params.panel)
            .and_then(|panel| Some((panel.title.clone(), panel.entry.clone()?)))
            .ok_or_else(|| {
                ActionError::InvalidParams(format!(
                    "{} contributes no web panel {}",
                    params.extension, params.panel
                ))
            })?;

        Ok(PanelInfo {
            extension: params.extension,
            panel: params.panel,
            title,
            entry,
            csp: CSP,
        })
    }
}
