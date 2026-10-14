use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Installed {
    pub id: String,
    pub version: String,
    pub sha256: String,
    pub manifest_sha256: String,
    pub signed: bool,
    pub enabled: bool,
    pub installed_at: u64,
}
