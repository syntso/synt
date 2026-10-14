use serde::de::DeserializeOwned;
use serde_json::Value;

use super::RpcError;

pub(super) fn read<T: DeserializeOwned>(params: &Value) -> Result<T, RpcError> {
    T::deserialize(params).map_err(|error| RpcError::invalid_params(error.to_string()))
}

pub(super) fn take<T: DeserializeOwned>(params: Value) -> Result<T, RpcError> {
    serde_json::from_value(params).map_err(|error| RpcError::invalid_params(error.to_string()))
}
