use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::RpcError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Result(Value),
    Error(RpcError),
}

impl Outcome {
    pub fn into_result(self) -> Result<Value, RpcError> {
        match self {
            Self::Result(value) => Ok(value),
            Self::Error(error) => Err(error),
        }
    }
}

impl From<Result<Value, RpcError>> for Outcome {
    fn from(result: Result<Value, RpcError>) -> Self {
        match result {
            Ok(value) => Self::Result(value),
            Err(error) => Self::Error(error),
        }
    }
}
