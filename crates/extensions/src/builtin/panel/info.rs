use serde::Serialize;
use serde_json::{Value, json};
use synt_proto::OneLine;

use crate::manifest::{AssetPath, LocalId, Panel};

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' \
     'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'none'; \
     frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; \
     frame-ancestors 'self'";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PanelInfo {
    extension: String,
    panel: LocalId,
    title: OneLine,
    entry: AssetPath,
    csp: &'static str,
}

impl PanelInfo {
    pub(crate) fn of(extension: String, panel: &Panel) -> Option<Self> {
        Some(Self {
            extension,
            panel: panel.id().clone(),
            title: panel.title().clone(),
            entry: panel.entry()?.clone(),
            csp: CSP,
        })
    }

    pub(crate) fn summary(&self) -> Value {
        json!({ "extension": self.extension, "panel": self.panel })
    }
}
