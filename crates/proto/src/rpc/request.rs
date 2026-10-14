use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::json_rpc::JsonRpc;
use super::{RequestId, RpcError, params};
use crate::Method;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    jsonrpc: JsonRpc,
    pub id: RequestId,
    pub method: String,

    #[serde(default)]
    pub params: Value,
}

impl Request {
    pub fn new(
        id: RequestId,
        method: Method,
        params: &impl Serialize,
    ) -> Result<Self, serde_json::Error> {
        let params = serde_json::to_value(params)?;

        Ok(Self {
            jsonrpc: JsonRpc,
            id,
            method: method.name().to_owned(),
            params,
        })
    }

    pub fn method(&self) -> Option<Method> {
        Method::parse(&self.method)
    }

    pub fn params<T: DeserializeOwned>(&self) -> Result<T, RpcError> {
        params::read(&self.params)
    }

    pub fn into_params<T: DeserializeOwned>(self) -> Result<T, RpcError> {
        params::take(self.params)
    }
}
