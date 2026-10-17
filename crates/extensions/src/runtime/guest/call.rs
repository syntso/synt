#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuestCall {
    action: String,
    target: Option<String>,
    params: String,
}

impl GuestCall {
    pub(crate) fn new(action: String, target: Option<String>, params: String) -> Self {
        Self {
            action,
            target,
            params,
        }
    }

    pub(crate) fn action(&self) -> &str {
        &self.action
    }

    pub(crate) fn target(&self) -> Option<&str> {
        self.target.as_deref()
    }

    pub(crate) fn params(&self) -> &str {
        &self.params
    }
}
