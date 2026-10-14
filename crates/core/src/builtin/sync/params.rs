use schemars::JsonSchema;
use serde::Deserialize;
use synt_sync::MemberId;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectParams {
    pub project: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MemberParams {
    pub project: String,
    #[schemars(with = "String")]
    pub member: MemberId,
}
