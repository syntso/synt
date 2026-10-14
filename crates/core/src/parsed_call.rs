use serde_json::{Value, json};
use synt_actions::{Handle, Registry};
use synt_journal::{ActionId, Details, ResourceRef, Seq};
use synt_proto::{ActionCall, RpcError};

const WILDCARDS: [char; 2] = ['*', '?'];

pub(crate) struct ParsedCall {
    pub(crate) action: ActionId,
    pub(crate) handle: Option<Handle>,
    pub(crate) target: Option<ResourceRef>,
    pub(crate) details: Details,
    pub(crate) params: Value,
    pub(crate) shown_params: Value,
}

impl ParsedCall {
    pub(crate) fn from_call(
        call: ActionCall,
        registry: &Registry,
        parent: Option<Seq>,
    ) -> Result<Self, RpcError> {
        let action = ActionId::new(&call.action)
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        let target = call
            .target
            .map(ResourceRef::new)
            .transpose()
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        if let Some(target) = &target
            && target.as_str().contains(WILDCARDS)
        {
            return Err(RpcError::invalid_params(format!(
                "the target {target} holds * or ?; a target names one thing"
            )));
        }

        let handle = registry.get(&action);
        let shown_params = handle.as_ref().map_or_else(
            || call.params.clone(),
            |handle| handle.recorded_params(&call.params),
        );

        let recorded = match parent {
            Some(parent) => json!({ "params": &shown_params, "parent": parent }),
            None => json!({ "params": &shown_params }),
        };
        let details = Details::from_serialize(&recorded).map_err(|error| {
            RpcError::invalid_params(format!("parameters cannot be recorded: {error}"))
        })?;

        Ok(Self {
            action,
            handle,
            target,
            details,
            params: call.params,
            shown_params,
        })
    }
}
