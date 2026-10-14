use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::version::JsonRpcVersion;
use super::{Notice, RpcError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    jsonrpc: JsonRpcVersion,
    pub method: String,

    #[serde(default)]
    pub params: Value,
}

impl Notification {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: JsonRpcVersion,
            method: method.into(),
            params,
        }
    }

    pub fn params<T: DeserializeOwned>(&self) -> Result<T, RpcError> {
        T::deserialize(&self.params).map_err(|error| RpcError::invalid_params(error.to_string()))
    }

    pub fn read<T: Notice>(&self) -> Option<Result<T, RpcError>> {
        if self.method != T::METHOD {
            return None;
        }

        Some(self.params())
    }
}
