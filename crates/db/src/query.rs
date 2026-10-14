use serde_json::Value;

use crate::{Access, Page};

#[derive(Debug, Clone, Copy)]
pub struct Query<'a> {
    pub text: &'a str,
    pub params: &'a [Value],
    pub access: Access,
    pub page: Page,
}
