use serde::{Deserialize, Serialize};

use super::version::JsonRpcVersion;
use super::{Outcome, RequestId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    jsonrpc: JsonRpcVersion,
    pub id: Option<RequestId>,

    #[serde(flatten)]
    pub outcome: Outcome,
}

impl Response {
    pub fn new(id: Option<RequestId>, outcome: impl Into<Outcome>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion,
            id,
            outcome: outcome.into(),
        }
    }
}
