use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::QueryParams;
use super::statement::Statement;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct QueryFields {
    #[serde(default)]
    pub(super) sql: Option<String>,

    #[serde(default)]
    pub(super) command: Option<Vec<String>>,

    #[serde(default)]
    pub(super) params: Vec<Value>,

    #[serde(default)]
    pub(super) limit: Option<usize>,

    #[serde(default)]
    pub(super) max_bytes: Option<usize>,

    #[serde(default)]
    pub(super) offset: u64,

    #[serde(default)]
    pub(super) timeout_seconds: Option<u64>,
}

impl From<QueryParams> for QueryFields {
    fn from(params: QueryParams) -> Self {
        let mut fields = Self {
            sql: None,
            command: None,
            params: Vec::new(),
            limit: params.limit,
            max_bytes: params.max_bytes,
            offset: params.offset,
            timeout_seconds: params.timeout_seconds,
        };

        match params.statement {
            Statement::Sql { sql, params } => {
                fields.sql = Some(sql);
                fields.params = params;
            }
            Statement::Command { command } => fields.command = Some(command),
        }

        fields
    }
}
