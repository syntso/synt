mod admitted;
mod invocation;
mod or_unavailable;
mod parsed_call;
mod refusal;

use admitted::Admitted;
use invocation::Invocation;
use or_unavailable::OrUnavailable;
use parsed_call::ParsedCall;
use refusal::Refusal;

use std::sync::Arc;

use serde_json::Value;
use synt_actions::{ActionError, Handle, Prepared, Registry};
use synt_journal::{ActionId, Actor, EventKind, JournalWriter, NewEvent, ResourceRef, Seq};
use synt_proto::{
    ActionCall, ActionDescription, ActionOutcome, ApprovalAnswer, ErrorCode, RequestSeq, RpcError,
};
use tokio::task;
use tokio_util::sync::CancellationToken;

use crate::access::{AccessRequest, Decision, Gate};
use crate::broker::Broker;
use crate::link::Link;
use crate::on_behalf::OnBehalf;
use crate::recorder::{Call, Recorder, Rejection};

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

    pub(crate) async fn dispatch_unprompted(
        &self,
        actor: &Actor,
        call: ActionCall,
    ) -> Result<ActionOutcome, RpcError> {
        let departed = CancellationToken::new();
        let link = Arc::new(Link::detached());

        self.dispatch_for(actor, None, call, &departed, &link).await
    }

    pub(crate) fn describe(&self, actor: &Actor) -> Vec<ActionDescription> {
        let for_person = actor.acts_for_person();

        self.registry
            .describe()
            .into_iter()
            .filter(|description| for_person || !description.person_only)
            .collect()
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

    pub(crate) fn action(&self, id: &ActionId) -> Option<Handle> {
        self.registry.get(id)
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
        let parsed = ParsedCall::from_call(call, &self.registry, parent)?;

        let call = Call {
            actor,
            action: &parsed.action,
            target: parsed.target.as_ref(),
        };
        let request = call.request(parsed.details);

        let prepared = match Self::screen(&call, parsed.handle, parsed.params).await {
            Ok(prepared) => prepared,
            Err(refusal) => return Err(self.reject(request, Self::refused(&call, refusal)).await),
        };

        let access = AccessRequest {
            actor,
            origin: origin.map(|origin| origin.actor),
            action: call.action,
            risk: prepared.risk(),
            target: call.target,
            destination: prepared.destination(),
            grantable: prepared.grantable(),
            params: prepared.shown().unwrap_or(&parsed.shown_params),
            run_approved: origin.is_some_and(|origin| origin.approves(prepared.destination())),
            departed,
        };
        let admitted = self.admit(&call, request, &access).await?;

        let invocation = Invocation {
            link,
            halt: origin.and_then(|origin| origin.stop),
        };

        self.perform(&call, admitted, prepared, invocation).await
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

        if handle.person_only() && !call.actor.acts_for_person() {
            let reason = format!("{} is for the person only", call.action);
            return Err(Refusal::Denied(reason));
        }

        let prepared = Self::prepare(&handle, call.target, params).await?;

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

    async fn prepare(
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

    async fn admit(
        &self,
        call: &Call<'_>,
        request: NewEvent,
        access: &AccessRequest<'_>,
    ) -> Result<Admitted, RpcError> {
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
    ) -> Result<Admitted, RpcError> {
        let grant = decision.grant();

        if let Decision::Deny { reason } = decision {
            return Err(self.reject(request, call.denied(reason)).await);
        }

        let steps = call.admission(&decision);
        let request = self.recorder.open(request, steps).await.or_unavailable()?;

        Ok(Admitted { request, grant })
    }

    async fn admit_after_asking(
        &self,
        call: &Call<'_>,
        request: Seq,
        decision: Decision,
    ) -> Result<Admitted, RpcError> {
        let grant = decision.grant();

        if let Decision::Deny { reason } = decision {
            return Err(self.conclude(request, call.denied(reason)).await);
        }

        let steps = call.admission(&decision);
        self.recorder
            .follow_up(request, steps)
            .await
            .or_unavailable()?;

        Ok(Admitted { request, grant })
    }

    async fn perform(
        &self,
        call: &Call<'_>,
        admitted: Admitted,
        prepared: Prepared,
        invocation: Invocation<'_>,
    ) -> Result<ActionOutcome, RpcError> {
        let Admitted { request, grant } = admitted;
        let finishes_later = prepared.finishes_later();
        let stop = invocation.stop();
        let hold = grant.and_then(|grant| self.broker.hold(grant, &stop));

        if stop.is_cancelled() {
            let refusal = Refusal::from(invocation.stopped());
            return Err(self.conclude(request, Self::refused(call, refusal)).await);
        }

        let context = invocation.context(call, request, &prepared, &stop, hold);

        let completed = match invocation.run(prepared, context, &stop).await {
            Ok(completed) => completed,
            Err(error) => {
                let refusal = Refusal::from(error);
                return Err(self.conclude(request, Self::refused(call, refusal)).await);
            }
        };

        if !finishes_later {
            let finished = call.step(EventKind::Finished, &completed.summary);
            self.recorder
                .follow_up(request, vec![finished])
                .await
                .or_unavailable()?;
        }

        Ok(ActionOutcome {
            request: RequestSeq::new(request.get()),
            value: completed.value,
        })
    }

    async fn reject(&self, request: NewEvent, rejection: Rejection) -> RpcError {
        let opened = self.recorder.open(request, vec![rejection.step]).await;

        opened.or_unavailable().err().unwrap_or(rejection.error)
    }

    async fn conclude(&self, request: Seq, rejection: Rejection) -> RpcError {
        let recorded = self.recorder.follow_up(request, vec![rejection.step]).await;

        recorded.or_unavailable().err().unwrap_or(rejection.error)
    }

    fn refused(call: &Call<'_>, refusal: Refusal) -> Rejection {
        match refusal {
            Refusal::Failed(error) => call.failed(error),
            Refusal::Denied(reason) => call.denied(reason),
        }
    }
}
