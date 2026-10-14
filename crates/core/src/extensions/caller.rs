use std::sync::{Arc, Weak};

use serde_json::Value;
use synt_actions::Context;
use synt_extensions::{CallFailure, CallFuture, Caller, ExtensionCall};
use synt_journal::Actor;
use synt_proto::{ActionCall, ActionOutcome, ErrorCode, RpcError};

use crate::core::Core;
use crate::link::Link;
use crate::on_behalf::OnBehalf;

pub struct CoreCaller {
    core: Weak<Core>,
}

impl CoreCaller {
    pub fn new(core: &Arc<Core>) -> Self {
        Self {
            core: Arc::downgrade(core),
        }
    }

    async fn dispatch(
        &self,
        call: ExtensionCall,
        origin: Option<&Context>,
    ) -> Result<Value, CallFailure> {
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| CallFailure::new("unavailable", "the core is stopping"))?;

        let actor = Actor::Plugin(call.extension);
        let request = ActionCall {
            action: call.action,
            target: call.target,
            params: call.params,
        };

        let answer = match origin {
            Some(origin) => Self::on_behalf(&core, &actor, origin, request).await?,
            None => core.dispatch_unprompted(&actor, request).await,
        };

        answer
            .map(|outcome| outcome.value)
            .map_err(|error| CallFailure::new(code_name(error.code), error.message))
    }

    async fn on_behalf(
        core: &Core,
        actor: &Actor,
        origin: &Context,
        request: ActionCall,
    ) -> Result<Result<ActionOutcome, RpcError>, CallFailure> {
        let link = origin
            .connection::<Link>()
            .ok_or_else(|| CallFailure::new("unavailable", "the call came without a connection"))?;

        let behalf = OnBehalf {
            actor: origin.actor(),
            request: origin.request(),
            approved: false,
            planned: None,
            stop: Some(origin.stop()),
        };

        Ok(core
            .dispatch_on_behalf(actor, behalf, request, origin.stop(), &link)
            .await)
    }
}

impl Caller for CoreCaller {
    fn call<'a>(&'a self, call: ExtensionCall, origin: Option<&'a Context>) -> CallFuture<'a> {
        Box::pin(self.dispatch(call, origin))
    }
}

fn code_name(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::DENIED => "denied",
        ErrorCode::UNKNOWN_ACTION => "unknown_action",
        ErrorCode::INVALID_PARAMS => "invalid_params",
        ErrorCode::ACTION_FAILED => "failed",
        ErrorCode::JOURNAL_UNAVAILABLE => "journal_unavailable",
        _ => "internal",
    }
}
