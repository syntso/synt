use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::RequestSeq;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionOutcome {
    pub request: RequestSeq,
    pub value: Value,
}
