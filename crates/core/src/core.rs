use serde_json::{Value, json};
use synt_actions::{ActionError, Context, Handle, Registry};
use synt_journal::{Actor, EventKind, JournalError, JournalWriter, Seq};
use synt_proto::{ActionCall, ActionOutcome, ErrorCode, RequestSeq, RpcError};

use crate::access::{Decision, Gate};
use crate::parsed_call::ParsedCall;
use crate::recorder::{Call, Recorder};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Core<G> {
    registry: Registry,
    recorder: Recorder,
    gate: G,
}

impl<G: Gate> Core<G> {
    pub fn new(registry: Registry, journal: JournalWriter, gate: G) -> Self {
        Self {
            registry,
            recorder: Recorder::new(journal),
            gate,
        }
    }

    pub async fn dispatch(
        &self,
        actor: &Actor,
        call: ActionCall,
    ) -> Result<ActionOutcome, RpcError> {
        let parsed = ParsedCall::from_call(call)?;
        let call = Call {
            actor,
            action: &parsed.action,
            target: parsed.target.as_ref(),
        };

        let request = self
            .recorder
            .requested(&call, parsed.details)
            .await
            .map_err(|error| unavailable(&error))?;

        let handle = self.admit(&call, request).await?;

        self.perform(&call, request, handle, parsed.params).await
    }

    async fn admit(&self, call: &Call<'_>, request: Seq) -> Result<&Handle, RpcError> {
        let Some(handle) = self.registry.get(call.action) else {
            let error = RpcError::new(
                ErrorCode::UNKNOWN_ACTION,
                format!("no action named {}", call.action),
            );
            return Err(self.refuse(call, request, EventKind::Failed, error).await);
        };

        if let Decision::Deny { reason } = self.gate.decide(call.actor, handle).await {
            let error = RpcError::new(ErrorCode::DENIED, reason);
            return Err(self.refuse(call, request, EventKind::Denied, error).await);
        }

        Ok(handle)
    }

    async fn perform(
        &self,
        call: &Call<'_>,
        request: Seq,
        handle: &Handle,
        params: Value,
    ) -> Result<ActionOutcome, RpcError> {
        self.mark(call, request, EventKind::Started).await?;

        let context = Context::new(call.actor.clone(), request, call.target.cloned());
        let ran = run_isolated(handle, context, params).await;

        match ran {
            Ok(value) => {
                self.mark(call, request, EventKind::Finished).await?;

                Ok(ActionOutcome {
                    request: RequestSeq(request.get()),
                    value,
                })
            }
            Err(error) => {
                let error = failure(&error);
                Err(self.refuse(call, request, EventKind::Failed, error).await)
            }
        }
    }

    async fn mark(&self, call: &Call<'_>, request: Seq, kind: EventKind) -> Result<(), RpcError> {
        self.recorder
            .follow_up(call, request, kind, &Value::Null)
            .await
            .map(|_| ())
            .map_err(|error| unavailable(&error))
    }

    async fn refuse(
        &self,
        call: &Call<'_>,
        request: Seq,
        kind: EventKind,
        error: RpcError,
    ) -> RpcError {
        let details = json!({ "error": &error.message });

        match self.recorder.follow_up(call, request, kind, &details).await {
            Ok(_) => error,
            Err(journal) => unavailable(&journal),
        }
    }
}

async fn run_isolated(
    handle: &Handle,
    context: Context,
    params: Value,
) -> Result<Value, ActionError> {
    let handle = handle.clone();
    let running = tokio::spawn(async move { handle.call(&context, params).await });

    running
        .await
        .unwrap_or_else(|_| Err(ActionError::failed("the action stopped unexpectedly")))
}

fn unavailable(error: &JournalError) -> RpcError {
    eprintln!("synt-core: journal: {error}");
    RpcError::new(ErrorCode::JOURNAL_UNAVAILABLE, "the journal is unavailable")
}

fn failure(error: &ActionError) -> RpcError {
    let code = match error {
        ActionError::InvalidParams(_) | ActionError::TargetRequired => ErrorCode::INVALID_PARAMS,
        ActionError::Failed(_) => ErrorCode::ACTION_FAILED,
        ActionError::InvalidId { .. } | ActionError::Duplicate(_) => ErrorCode::INTERNAL,
    };

    RpcError::new(code, error.to_string())
}
