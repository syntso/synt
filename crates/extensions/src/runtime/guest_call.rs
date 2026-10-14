#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestCall {
    pub(crate) action: String,
    pub(crate) target: Option<String>,
    pub(crate) params: String,
}
