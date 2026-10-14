use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PanelInfo {
    pub extension: String,
    pub panel: String,
    pub title: String,
    pub entry: String,
    pub csp: &'static str,
}
