use serde_json::{Value, json};
use synt_actions::{RegisteredAction, Registry};
use synt_journal::{ActionId, Details, ResourceRef, Seq};
use synt_proto::{ActionCall, RpcError};

const WILDCARDS: [char; 2] = ['*', '?'];

pub(super) struct ParsedCall {
    pub(super) action: ActionId,
    pub(super) handle: Option<RegisteredAction>,
    pub(super) target: Option<ResourceRef>,
    pub(super) details: Details,
    pub(super) params: Value,
    pub(super) shown_params: Value,
}

impl ParsedCall {
    pub(super) fn from_call(
        call: ActionCall,
        registry: &Registry,
        parent: Option<Seq>,
    ) -> Result<Self, RpcError> {
        let action = ActionId::new(&call.action)
            .map_err(|error| RpcError::invalid_params(error.to_string()))?;

        let target = call.target.map(Self::one_target).transpose()?;

        let handle = registry.get(&action);
        let shown_params = handle.as_ref().map_or_else(
            || call.params.clone(),
            |handle| handle.recorded_params(&call.params),
        );
        let details = Self::details(&shown_params, parent)?;

        Ok(Self {
            action,
            handle,
            target,
            details,
            params: call.params,
            shown_params,
        })
    }

    fn one_target(text: String) -> Result<ResourceRef, RpcError> {
        let target =
            ResourceRef::new(text).map_err(|error| RpcError::invalid_params(error.to_string()))?;

        if target.as_str().contains(WILDCARDS) {
            return Err(RpcError::invalid_params(format!(
                "the target {target} holds * or ?; a target names one thing"
            )));
        }

        Ok(target)
    }

    fn details(shown_params: &Value, parent: Option<Seq>) -> Result<Details, RpcError> {
        let recorded = match parent {
            Some(parent) => json!({ "params": shown_params, "parent": parent }),
            None => json!({ "params": shown_params }),
        };

        Details::from_serialize(&recorded).map_err(|error| {
            RpcError::invalid_params(format!("parameters cannot be recorded: {error}"))
        })
    }
}
