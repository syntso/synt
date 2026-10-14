use super::Namespaces;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListQuery {
    pub namespaces: Namespaces,
    pub selector: Option<String>,
    pub limit: Option<u32>,
    pub continue_token: Option<String>,
}
