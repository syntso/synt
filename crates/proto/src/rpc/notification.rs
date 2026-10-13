use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::RpcError;
use super::json_rpc::JsonRpc;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    jsonrpc: JsonRpc,
    pub method: String,

    #[serde(default)]
    pub params: Value,
}

impl Notification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: JsonRpc,
            method: method.into(),
            params,
        }
    }

    pub fn params<T: DeserializeOwned>(&self) -> Result<T, RpcError> {
        T::deserialize(&self.params).map_err(|error| RpcError::invalid_params(error.to_string()))
    }
}
