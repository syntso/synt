#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewGrant {
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) resource: Option<String>,
    pub(crate) destination: Option<String>,
}
