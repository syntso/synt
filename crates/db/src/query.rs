use serde_json::Value;

use crate::{Access, Page};

#[derive(Debug, Clone, Copy)]
pub struct Query<'a> {
    pub text: &'a str,
    pub params: &'a [Value],
    pub access: Access,
    pub page: Page,
}

impl<'a> Query<'a> {
    pub(crate) fn for_one_value(text: &'a str, params: &'a [Value]) -> Self {
        Self {
            text,
            params,
            access: Access::ReadOnly,
            page: Page::new(1, usize::MAX),
        }
    }
}
