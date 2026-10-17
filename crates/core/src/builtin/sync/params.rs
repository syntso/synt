use schemars::JsonSchema;
use serde::Deserialize;
use synt_sync::MemberId;

use crate::projects::ProjectSlug;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectParams {
    pub(crate) project: ProjectSlug,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemberParams {
    pub(crate) project: ProjectSlug,
    #[schemars(with = "String")]
    pub(crate) member: MemberId,
}
