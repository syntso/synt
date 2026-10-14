use std::sync::{Arc, Weak};

use serde_json::Value;
use synt_actions::Context;
use synt_extensions::{CallFailure, CallFuture, Caller, ExtensionCall, FailureCode};
use synt_journal::Actor;
use synt_proto::{ActionCall, ActionOutcome, RpcError};

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
            .ok_or_else(|| CallFailure::new(FailureCode::Unavailable, "the core is stopping"))?;

        let actor = Actor::Plugin(call.extension().clone());
        let request = call.into_request();

        let answer = match origin {
            Some(origin) => Self::on_behalf(&core, &actor, origin, request).await?,
            None => core.dispatch_unprompted(&actor, request).await,
        };

        answer
            .map(|outcome| outcome.value)
            .map_err(|error| CallFailure::new(error.code.into(), error.message))
    }

    async fn on_behalf(
        core: &Core,
        actor: &Actor,
        origin: &Context,
        request: ActionCall,
    ) -> Result<Result<ActionOutcome, RpcError>, CallFailure> {
        let link = origin.connection::<Link>().ok_or_else(|| {
            CallFailure::new(
                FailureCode::Unavailable,
                "the call came without a connection",
            )
        })?;

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
