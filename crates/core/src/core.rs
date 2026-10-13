use std::any::Any;
use std::sync::Arc;

use serde_json::{Value, json};
use synt_actions::{ActionError, Completed, Context, Handle, Prepared, Registry};
use synt_journal::{Actor, EventKind, JournalError, JournalWriter, NewEvent, ResourceRef, Seq};
use synt_proto::{ActionCall, ActionOutcome, ApprovalAnswer, ErrorCode, RequestSeq, RpcError};
use tokio::task;
use tokio_util::sync::CancellationToken;

use crate::access::{AccessRequest, Decision, Gate, Scope};
use crate::broker::Broker;
use crate::link::Link;
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

    pub(crate) async fn dispatch(
        &self,
        actor: &Actor,
        call: ActionCall,
        departed: &CancellationToken,
        link: &Arc<Link>,
    ) -> Result<ActionOutcome, RpcError> {
        let ParsedCall {
            action,
            handle,
            target,
            details,
            params,
            shown_params,
        } = ParsedCall::from_call(call, &self.registry)?;

        let call = Call {
            actor,
            action: &action,
            target: target.as_ref(),
        };
        let request = Recorder::request(&call, details);

        let Some(handle) = handle else {
            let error = RpcError::new(
                ErrorCode::UNKNOWN_ACTION,
                format!("no action named {action}"),
            );
            return Err(self.reject(request, failed(&call, error)).await);
        };

        if handle.person_only() && !acts_for_person(actor) {
            let reason = format!("{action} is for the person only");
            return Err(self.reject(request, denied(&call, reason)).await);
        }

        let prepared = match prepare_blocking(handle, call.target, params).await {
            Ok(prepared) => prepared,
            Err(error) => return Err(self.reject(request, failed(&call, failure(&error))).await),
        };

        let access = AccessRequest {
            actor,
            action: &action,
            risk: prepared.risk(),
            target: call.target,
            params: &shown_params,
            departed,
        };
        let request = self.admit(&call, request, &access).await?;

        self.perform(&call, request, prepared, link).await
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

    async fn admit(
        &self,
        call: &Call<'_>,
        request: NewEvent,
        access: &AccessRequest<'_>,
    ) -> Result<Seq, RpcError> {
        if let Some(decision) = self.broker.settled(access) {
            return self.admit_at_once(call, request, decision).await;
        }

        let request = self
            .recorder
            .open(request, Vec::new())
            .await
            .map_err(|error| unavailable(&error))?;

        let decision = self.broker.decide(access).await;

        self.admit_after_asking(call, request, decision).await
    }

    async fn admit_at_once(
        &self,
        call: &Call<'_>,
        request: NewEvent,
        decision: Decision,
    ) -> Result<Seq, RpcError> {
        let steps = match decision {
            Decision::Deny { reason } => {
                return Err(self.reject(request, denied(call, reason)).await);
            }
            Decision::Allow => admission(call, None),
            Decision::Approved(scope) => admission(call, Some(scope)),
        };

        self.recorder
            .open(request, steps)
            .await
            .map_err(|error| unavailable(&error))
    }

    async fn admit_after_asking(
        &self,
        call: &Call<'_>,
        request: Seq,
        decision: Decision,
    ) -> Result<Seq, RpcError> {
        let steps = match decision {
            Decision::Deny { reason } => {
                return Err(self.conclude(request, denied(call, reason)).await);
            }
            Decision::Allow => admission(call, None),
            Decision::Approved(scope) => admission(call, Some(scope)),
        };

        self.recorder
            .follow_up(request, steps)
            .await
            .map(|()| request)
            .map_err(|error| unavailable(&error))
    }

    async fn perform(
        &self,
        call: &Call<'_>,
        request: Seq,
        prepared: Prepared,
        link: &Arc<Link>,
    ) -> Result<ActionOutcome, RpcError> {
        let finishes_later = prepared.finishes_later();
        let connection: Arc<dyn Any + Send + Sync> = Arc::clone(link) as _;
        let context = Context::new(call.actor.clone(), request, call.target.cloned())
            .with_risk(prepared.risk())
            .with_connection(connection);

        let completed = match run_isolated(prepared, context).await {
            Ok(completed) => completed,
            Err(error) => return Err(self.conclude(request, failed(call, failure(&error))).await),
        };

        if !finishes_later {
            let finished = Recorder::step(call, EventKind::Finished, &completed.summary);
            self.recorder
                .follow_up(request, vec![finished])
                .await
                .map_err(|error| unavailable(&error))?;
        }

        Ok(ActionOutcome {
            request: RequestSeq(request.get()),
            value: completed.value,
        })
    }

    async fn reject(&self, request: NewEvent, (step, error): (NewEvent, RpcError)) -> RpcError {
        match self.recorder.open(request, vec![step]).await {
            Ok(_) => error,
            Err(journal) => unavailable(&journal),
        }
    }

    async fn conclude(&self, request: Seq, (step, error): (NewEvent, RpcError)) -> RpcError {
        match self.recorder.follow_up(request, vec![step]).await {
            Ok(()) => error,
            Err(journal) => unavailable(&journal),
        }
    }
}

fn admission(call: &Call<'_>, approval: Option<Scope>) -> Vec<NewEvent> {
    let started = Recorder::step(call, EventKind::Started, &Value::Null);

    let Some(scope) = approval else {
        return vec![started];
    };

    let details = json!({ "scope": scope, "by": "person" });
    let approved = Recorder::step(call, EventKind::Approved, &details);

    vec![approved, started]
}

fn denied(call: &Call<'_>, reason: String) -> (NewEvent, RpcError) {
    let details = json!({ "reason": &reason });
    let step = Recorder::step(call, EventKind::Denied, &details);

    (step, RpcError::new(ErrorCode::DENIED, reason))
}

fn failed(call: &Call<'_>, error: RpcError) -> (NewEvent, RpcError) {
    let details = json!({ "error": &error.message });
    let step = Recorder::step(call, EventKind::Failed, &details);

    (step, error)
}

async fn prepare_blocking(
    handle: &Handle,
    target: Option<&ResourceRef>,
    params: Value,
) -> Result<Prepared, ActionError> {
    let handle = handle.clone();
    let target = target.cloned();

    let preparing = task::spawn_blocking(move || handle.prepare(target.as_ref(), params));

    preparing.await.unwrap_or_else(|_| {
        Err(ActionError::failed(
            "preparing the call stopped unexpectedly",
        ))
    })
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

fn acts_for_person(actor: &Actor) -> bool {
    matches!(actor, Actor::Person | Actor::Core)
}
