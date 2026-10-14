use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum ScreenUpdate {
    ViewPatched {
        extension: String,
        view: String,
        patch: Value,
    },
    PanelPosted {
        extension: String,
        panel: String,
        message: Value,
    },
}
