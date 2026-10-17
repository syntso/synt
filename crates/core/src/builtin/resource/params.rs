use schemars::JsonSchema;
use serde::Deserialize;

use crate::projects::{ProjectSlug, ResourceName};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ResourceParams {
    pub(crate) project: ProjectSlug,
    pub(crate) name: ResourceName,
}
