use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};
use synt_actions::{ActionError, Completed, Context, Description, Handle, Prepared, Registry};
use synt_journal::{
    ActionId, Actor, EventKind, JournalError, JournalWriter, NewEvent, ResourceRef, Seq,
};
use synt_proto::{
    ActionCall, ActionDescription, ActionOutcome, ApprovalAnswer, ErrorCode, RequestSeq, RpcError,
};
use tokio::{task, time};
use tokio_util::sync::CancellationToken;

use crate::access::{AccessRequest, Decision, Gate, Scope};
use crate::broker::{Broker, risk_level};
use crate::grants::{GrantId, Hold};
use crate::link::Link;
use crate::log::log;
use crate::on_behalf::OnBehalf;
use crate::parsed_call::ParsedCall;
use crate::recorder::{Call, Recorder};
use crate::refusal::Refusal;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const STOP_GRACE: Duration = Duration::from_secs(10);
const REVOKED: &str = "stopped: the standing grant this call used was revoked";
const HALTED: &str = "stopped: the work this call belongs to was stopped";

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
        self.dispatch_for(actor, None, call, departed, link).await
    }

    pub(crate) async fn dispatch_on_behalf(
        &self,
        actor: &Actor,
        origin: OnBehalf<'_>,
        call: ActionCall,
        departed: &CancellationToken,
        link: &Arc<Link>,
    ) -> Result<ActionOutcome, RpcError> {
        self.dispatch_for(actor, Some(origin), call, departed, link)
            .await
    }

    async fn dispatch_for(
        &self,
        actor: &Actor,
        origin: Option<OnBehalf<'_>>,
        call: ActionCall,
        departed: &CancellationToken,
        link: &Arc<Link>,
    ) -> Result<ActionOutcome, RpcError> {
        let parent = origin.map(|origin| origin.request);
        let halt = origin.and_then(|origin| origin.stop);
        let ParsedCall {
            action,
            handle,
            target,
            details,
            params,
            shown_params,
        } = ParsedCall::from_call(call, &self.registry, parent)?;

        let call = Call {
            actor,
            action: &action,
            target: target.as_ref(),
        };
        let request = Recorder::request(&call, details);

        let prepared = match screen(&call, handle, params).await {
            Ok(prepared) => prepared,
            Err(refusal) => return Err(self.reject(request, recorded(&call, refusal)).await),
        };

        let access = AccessRequest {
            actor,
            origin: origin.map(|origin| origin.actor),
            action: &action,
            risk: prepared.risk(),
            target: call.target,
            destination: prepared.destination(),
            grantable: prepared.grantable(),
            params: prepared.shown().unwrap_or(&shown_params),
            run_approved: origin.is_some_and(|origin| origin.approves(prepared.destination())),
            departed,
        };
        let (request, grant) = self.admit(&call, request, &access).await?;

        self.perform(&call, (request, grant), prepared, (link, halt))
            .await
    }

    pub(crate) fn describe(&self, actor: &Actor) -> Vec<ActionDescription> {
        let for_person = acts_for_person(actor);

        self.registry
            .describe()
            .into_iter()
            .filter(|description| for_person || !description.person_only)
            .map(action_description)
            .collect()
    }

    pub(crate) fn respond(
        &self,
        responder: &Actor,
        answer: ApprovalAnswer,
    ) -> Result<(), RpcError> {
        self.broker.answer(responder, answer)
    }

    pub(crate) async fn dispatch_unprompted(
        &self,
        actor: &Actor,
        call: ActionCall,
    ) -> Result<ActionOutcome, RpcError> {
        let departed = CancellationToken::new();
        let link = Arc::new(Link::detached());

        self.dispatch_for(actor, None, call, &departed, &link).await
    }

    pub(crate) fn broker(&self) -> &Broker {
        &self.broker
    }

    pub(crate) fn action(&self, id: &ActionId) -> Option<Handle> {
        self.registry.get(id)
    }

    async fn admit(
        &self,
        call: &Call<'_>,
        request: NewEvent,
        access: &AccessRequest<'_>,
    ) -> Result<(Seq, Option<GrantId>), RpcError> {
        if let Some(decision) = self.broker.settled(access) {
            return self.admit_at_once(call, request, decision).await;
        }

        let request = self
            .recorder
            .open(request, Vec::new())
            .await
            .or_unavailable()?;

        let decision = self.broker.decide(access).await;

        self.admit_after_asking(call, request, decision).await
    }

    async fn admit_at_once(
        &self,
        call: &Call<'_>,
        request: NewEvent,
        decision: Decision,
    ) -> Result<(Seq, Option<GrantId>), RpcError> {
        let grant = decision.grant();
        let steps = match decision {
            Decision::Deny { reason } => {
                return Err(self.reject(request, denied(call, reason)).await);
            }
            admitted => admission(call, &admitted),
        };

        self.recorder
            .open(request, steps)
            .await
            .map(|request| (request, grant))
            .or_unavailable()
    }

    async fn admit_after_asking(
        &self,
        call: &Call<'_>,
        request: Seq,
        decision: Decision,
    ) -> Result<(Seq, Option<GrantId>), RpcError> {
        let grant = decision.grant();
        let steps = match decision {
            Decision::Deny { reason } => {
                return Err(self.conclude(request, denied(call, reason)).await);
            }
            admitted => admission(call, &admitted),
        };

        self.recorder
            .follow_up(request, steps)
            .await
            .map(|()| (request, grant))
            .or_unavailable()
    }

    async fn perform(
        &self,
        call: &Call<'_>,
        (request, grant): (Seq, Option<GrantId>),
        prepared: Prepared,
        (link, halt): (&Arc<Link>, Option<&CancellationToken>),
    ) -> Result<ActionOutcome, RpcError> {
        let finishes_later = prepared.finishes_later();
        let stop = halt.map_or_else(CancellationToken::new, CancellationToken::child_token);
        let hold = grant.and_then(|grant| self.broker.hold(grant, &stop));

        if stop.is_cancelled() {
            let error = ActionError::failed(stopped(halt));
            return Err(self.conclude(request, failed(call, failure(&error))).await);
        }

        let context = context(call, request, &prepared, link, &stop, hold);

        let completed = match run_isolated(prepared, context, (&stop, halt)).await {
            Ok(completed) => completed,
            Err(error) => return Err(self.conclude(request, failed(call, failure(&error))).await),
        };

        if !finishes_later {
            let finished = Recorder::step(call, EventKind::Finished, &completed.summary);
            self.recorder
                .follow_up(request, vec![finished])
                .await
                .or_unavailable()?;
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

async fn screen(
    call: &Call<'_>,
    handle: Option<Handle>,
    params: Value,
) -> Result<Prepared, Refusal> {
    let Some(handle) = handle else {
        let error = RpcError::new(
            ErrorCode::UNKNOWN_ACTION,
            format!("no action named {}", call.action),
        );
        return Err(Refusal::Failed(error));
    };

    if handle.person_only() && !acts_for_person(call.actor) {
        let reason = format!("{} is for the person only", call.action);
        return Err(Refusal::Denied(reason));
    }

    let prepared = prepare_blocking(&handle, call.target, params)
        .await
        .map_err(|error| Refusal::Failed(failure(&error)))?;

    if let Some(target) = call
        .target
        .filter(|_| handle.takes_target() && prepared.destination().is_none())
    {
        let error = RpcError::new(
            ErrorCode::INVALID_PARAMS,
            format!("{target} does not name a host this core can reach"),
        );
        return Err(Refusal::Failed(error));
    }

    Ok(prepared)
}

fn recorded(call: &Call<'_>, refusal: Refusal) -> (NewEvent, RpcError) {
    match refusal {
        Refusal::Failed(error) => failed(call, error),
        Refusal::Denied(reason) => denied(call, reason),
    }
}

fn admission(call: &Call<'_>, decision: &Decision) -> Vec<NewEvent> {
    let started = Recorder::step(call, EventKind::Started, &Value::Null);

    let details = match decision {
        Decision::Approved(scope) => json!({ "scope": scope, "by": "person" }),
        Decision::Granted(grant) => {
            json!({ "scope": Scope::Always, "grant": grant, "by": "person" })
        }
        Decision::Allow | Decision::Deny { .. } => return vec![started],
    };

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

fn context(
    call: &Call<'_>,
    request: Seq,
    prepared: &Prepared,
    link: &Arc<Link>,
    stop: &CancellationToken,
    hold: Option<Hold>,
) -> Context {
    let connection: Arc<dyn Any + Send + Sync> = Arc::clone(link) as _;
    let destination = prepared.destination().map(str::to_owned);

    let context = Context::new(call.actor.clone(), request, call.target.cloned())
        .with_risk(prepared.risk())
        .with_destination(destination)
        .with_connection(connection)
        .with_stop(stop.clone());

    match hold {
        Some(hold) => context.with_hold(Arc::new(hold)),
        None => context,
    }
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

async fn run_isolated(
    prepared: Prepared,
    context: Context,
    (stop, halt): (&CancellationToken, Option<&CancellationToken>),
) -> Result<Completed, ActionError> {
    let mut running = tokio::spawn(async move { prepared.run(&context).await });

    tokio::select! {
        finished = &mut running => {
            return finished
                .unwrap_or_else(|_| Err(ActionError::failed("the action stopped unexpectedly")));
        }
        () = stop.cancelled() => {}
    }

    if time::timeout(STOP_GRACE, &mut running).await.is_err() {
        running.abort();
    }

    Err(ActionError::failed(stopped(halt)))
}

fn stopped(halt: Option<&CancellationToken>) -> &'static str {
    match halt {
        Some(halt) if halt.is_cancelled() => HALTED,
        _ => REVOKED,
    }
}

fn unavailable(error: &JournalError) -> RpcError {
    log!("journal: {error}");
    RpcError::new(ErrorCode::JOURNAL_UNAVAILABLE, "the journal is unavailable")
}

trait OrUnavailable<T> {
    fn or_unavailable(self) -> Result<T, RpcError>;
}

impl<T> OrUnavailable<T> for Result<T, JournalError> {
    fn or_unavailable(self) -> Result<T, RpcError> {
        self.map_err(|error| unavailable(&error))
    }
}

fn failure(error: &ActionError) -> RpcError {
    let code = match error {
        ActionError::InvalidParams(_) | ActionError::TargetRequired => ErrorCode::INVALID_PARAMS,
        ActionError::Failed(_) => ErrorCode::ACTION_FAILED,
        ActionError::InvalidId { .. } | ActionError::Duplicate(_) => ErrorCode::INTERNAL,
    };

    RpcError::new(code, error.to_string())
}

fn action_description(description: Description) -> ActionDescription {
    ActionDescription {
        id: description.id.as_str().to_owned(),
        title: description.title,
        risk: risk_level(description.risk),
        person_only: description.person_only,
        takes_target: description.takes_target,
        params: description.params,
    }
}

fn acts_for_person(actor: &Actor) -> bool {
    matches!(actor, Actor::Person | Actor::Core)
}
