use std::collections::BTreeMap;

use serde_json::Value;
use synt_actions::Registry;
use synt_journal::{ActionId, Details, ResourceRef};
use synt_proto::{ActionCall, RpcError};

pub(crate) struct ParsedCall {
    pub(crate) action: ActionId,
    pub(crate) target: Option<ResourceRef>,
    pub(crate) details: Details,
    pub(crate) params: Value,
    pub(crate) shown_params: Value,
}

impl ParsedCall {
    pub(crate) fn from_call(call: ActionCall, registry: &Registry) -> Result<Self, RpcError> {
        let action = ActionId::new(&call.action)
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        let target = call
            .target
            .map(ResourceRef::new)
            .transpose()
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        let shown_params = registry.get(&action).map_or_else(
            || call.params.clone(),
            |handle| handle.recorded_params(&call.params),
        );

        let recorded = BTreeMap::from([("params", &shown_params)]);
        let details = Details::from_serialize(&recorded).map_err(|error| {
            RpcError::invalid_params(format!("parameters cannot be recorded: {error}"))
        })?;

        Ok(Self {
            action,
            target,
            details,
            params: call.params,
            shown_params,
        })
    }
}
