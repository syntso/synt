use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::json_rpc::JsonRpc;
use super::{Notice, RpcError, params};

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
        params::read(&self.params)
    }

    pub fn read<T: Notice>(&self) -> Option<Result<T, RpcError>> {
        if self.method != T::METHOD {
            return None;
        }

        Some(self.params())
    }
}
