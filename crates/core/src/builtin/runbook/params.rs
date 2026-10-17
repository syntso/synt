use schemars::JsonSchema;
use serde::Deserialize;

use crate::projects::ProjectSlug;
use crate::runbooks::RunbookName;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunbookParams {
    pub(crate) project: ProjectSlug,
    pub(crate) name: RunbookName,
}
