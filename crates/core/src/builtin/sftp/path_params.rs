use schemars::JsonSchema;
use serde::Deserialize;

use super::RemotePath;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct PathParams {
    pub path: RemotePath,
}
