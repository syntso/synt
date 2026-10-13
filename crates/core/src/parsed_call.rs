use std::collections::BTreeMap;

use serde_json::Value;
use synt_journal::{ActionId, Details, ResourceRef};
use synt_proto::{ActionCall, RpcError};

pub(crate) struct ParsedCall {
    pub(crate) action: ActionId,
    pub(crate) target: Option<ResourceRef>,
    pub(crate) details: Details,
    pub(crate) params: Value,
}

impl ParsedCall {
    pub(crate) fn from_call(call: ActionCall) -> Result<Self, RpcError> {
        let action = ActionId::new(&call.action)
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        let target = call
            .target
            .map(ResourceRef::new)
            .transpose()
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        let recorded = BTreeMap::from([("params", &call.params)]);
        let details = Details::from_serialize(&recorded).map_err(|error| {
            RpcError::invalid_params(format!("parameters cannot be recorded: {error}"))
        })?;

        Ok(Self {
            action,
            target,
            details,
            params: call.params,
        })
    }
}
