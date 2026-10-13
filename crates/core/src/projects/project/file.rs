use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct ProjectFile {
    #[serde(default)]
    pub(super) id: Option<String>,

    #[serde(default)]
    pub(super) name: Option<String>,
}
