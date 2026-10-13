use serde::{Deserialize, Serialize};

use super::json_rpc::JsonRpc;
use super::{Outcome, RequestId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Response {
    jsonrpc: JsonRpc,
    pub id: Option<RequestId>,

    #[serde(flatten)]
    pub outcome: Outcome,
}

impl Response {
    pub fn new(id: Option<RequestId>, outcome: impl Into<Outcome>) -> Self {
        Self {
            jsonrpc: JsonRpc,
            id,
            outcome: outcome.into(),
        }
    }
}
