#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredGrant {
    pub(crate) id: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) resource: Option<String>,
    pub(crate) destination: Option<String>,
    pub(crate) created_at: u64,
}
