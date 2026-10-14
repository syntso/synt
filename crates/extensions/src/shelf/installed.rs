#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Installed {
    pub id: String,
    pub version: String,
    pub sha256: String,
    pub manifest_sha256: String,
    pub signed: bool,
    pub enabled: bool,
    pub added_at: u64,
}
