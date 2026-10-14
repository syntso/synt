use std::sync::{Arc, Weak};

use synt_actions::Context;
use synt_extensions::{CallFailure, CallFuture, Caller, ExtensionCall};
use synt_journal::Actor;
use synt_proto::{ActionCall, ErrorCode};

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
        context: &Context,
    ) -> Result<serde_json::Value, CallFailure> {
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| CallFailure::new("unavailable", "the core is stopping"))?;
        let link = context
            .connection::<Link>()
            .ok_or_else(|| CallFailure::new("unavailable", "the call came without a connection"))?;

        let actor = Actor::Plugin(call.extension);
        let request = ActionCall {
            action: call.action,
            target: call.target,
            params: call.params,
        };

        let origin = OnBehalf {
            actor: context.actor(),
            request: context.request(),
            approved: false,
            stop: Some(context.stop()),
        };
        let answer = core
            .dispatch_on_behalf(&actor, origin, request, context.stop(), &link)
            .await;

        match answer {
            Ok(outcome) => Ok(outcome.value),
            Err(error) => Err(CallFailure::new(code_name(error.code), error.message)),
        }
    }
}

impl Caller for CoreCaller {
    fn call<'a>(&'a self, call: ExtensionCall, context: &'a Context) -> CallFuture<'a> {
        Box::pin(self.dispatch(call, context))
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
