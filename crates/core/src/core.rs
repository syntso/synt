use serde_json::{Value, json};
use synt_actions::{ActionError, Completed, Context, Handle, Prepared, Registry, Risk};
use synt_journal::{Actor, EventKind, JournalError, JournalWriter, Seq};
use synt_proto::{ActionCall, ActionOutcome, ApprovalAnswer, ErrorCode, RequestSeq, RpcError};
use tokio_util::sync::CancellationToken;

use crate::access::{AccessRequest, Decision, Gate, Scope};
use crate::broker::Broker;
use crate::parsed_call::ParsedCall;
use crate::recorder::{Call, Recorder};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Core {
    registry: Registry,
    recorder: Recorder,
    broker: Broker,
}

impl Core {
    pub fn new(registry: Registry, journal: JournalWriter, broker: Broker) -> Self {
        Self {
            registry,
            recorder: Recorder::new(journal),
            broker,
        }
    }

    pub async fn dispatch(
        &self,
        actor: &Actor,
        call: ActionCall,
        departed: &CancellationToken,
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

        let handle = self.find(&call, request).await?;
        let prepared = self.prepare(&call, request, handle, &parsed.params).await?;

        self.admit(&call, request, prepared.risk(), &parsed.params, departed)
            .await?;

        self.perform(&call, request, prepared).await
    }

    pub(crate) fn respond(
        &self,
        responder: &Actor,
        answer: ApprovalAnswer,
    ) -> Result<(), RpcError> {
        self.broker.answer(responder, answer)
    }

    pub(crate) fn broker(&self) -> &Broker {
        &self.broker
    }

    async fn find(&self, call: &Call<'_>, request: Seq) -> Result<&Handle, RpcError> {
        if let Some(handle) = self.registry.get(call.action) {
            return Ok(handle);
        }

        let error = RpcError::new(
            ErrorCode::UNKNOWN_ACTION,
            format!("no action named {}", call.action),
        );
        Err(self.fail(call, request, error).await)
    }

    async fn prepare(
        &self,
        call: &Call<'_>,
        request: Seq,
        handle: &Handle,
        params: &Value,
    ) -> Result<Prepared, RpcError> {
        match handle.prepare(call.target, params.clone()) {
            Ok(prepared) => Ok(prepared),
            Err(error) => Err(self.fail(call, request, failure(&error)).await),
        }
    }

    async fn admit(
        &self,
        call: &Call<'_>,
        request: Seq,
        risk: Risk,
        params: &Value,
        departed: &CancellationToken,
    ) -> Result<(), RpcError> {
        let access = AccessRequest {
            actor: call.actor,
            action: call.action,
            risk,
            target: call.target,
            params,
            departed,
        };

        match self.broker.decide(&access).await {
            Decision::Allow => Ok(()),
            Decision::Approved(scope) => self.approve(call, request, scope).await,
            Decision::Deny { reason } => Err(self.deny(call, request, reason).await),
        }
    }

    async fn perform(
        &self,
        call: &Call<'_>,
        request: Seq,
        prepared: Prepared,
    ) -> Result<ActionOutcome, RpcError> {
        self.mark(call, request, EventKind::Started).await?;

        let context = Context::new(call.actor.clone(), request, call.target.cloned());
        let ran = run_isolated(prepared, context).await;

        match ran {
            Ok(completed) => {
                self.record(call, request, EventKind::Finished, &completed.summary)
                    .await?;

                Ok(ActionOutcome {
                    request: RequestSeq(request.get()),
                    value: completed.value,
                })
            }
            Err(error) => Err(self.fail(call, request, failure(&error)).await),
        }
    }

    async fn mark(&self, call: &Call<'_>, request: Seq, kind: EventKind) -> Result<(), RpcError> {
        self.record(call, request, kind, &Value::Null).await
    }

    async fn approve(&self, call: &Call<'_>, request: Seq, scope: Scope) -> Result<(), RpcError> {
        let details = json!({ "scope": scope, "by": "person" });
        self.record(call, request, EventKind::Approved, &details)
            .await
    }

    async fn deny(&self, call: &Call<'_>, request: Seq, reason: String) -> RpcError {
        let details = json!({ "reason": &reason });
        let error = RpcError::new(ErrorCode::DENIED, reason);

        match self
            .record(call, request, EventKind::Denied, &details)
            .await
        {
            Ok(()) => error,
            Err(journal) => journal,
        }
    }

    async fn fail(&self, call: &Call<'_>, request: Seq, error: RpcError) -> RpcError {
        let details = json!({ "error": &error.message });

        match self
            .record(call, request, EventKind::Failed, &details)
            .await
        {
            Ok(()) => error,
            Err(journal) => journal,
        }
    }

    async fn record(
        &self,
        call: &Call<'_>,
        request: Seq,
        kind: EventKind,
        details: &Value,
    ) -> Result<(), RpcError> {
        self.recorder
            .follow_up(call, request, kind, details)
            .await
            .map(|_| ())
            .map_err(|error| unavailable(&error))
    }
}

async fn run_isolated(prepared: Prepared, context: Context) -> Result<Completed, ActionError> {
    let running = tokio::spawn(async move { prepared.run(&context).await });

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
