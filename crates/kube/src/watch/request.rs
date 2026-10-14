use crate::Namespaces;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WatchRequest {
    pub namespaces: Namespaces,
    pub selector: Option<String>,
}
