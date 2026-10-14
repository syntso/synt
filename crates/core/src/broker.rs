use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use synt_journal::Actor;
use synt_proto::{
    ApprovalAnswer, ApprovalDecision, ApprovalId, ErrorCode, Message, Notice, Notification,
    RpcError,
};
use tokio::sync::mpsc;
use tokio::time;
use tokio_util::sync::CancellationToken;

use super::apps::Apps;
use super::grant::Grant;
use super::grants::Grants;
use super::notice;
use super::registration::Registration;
use super::rule::Rule;
use super::waiting::Waiting;
use crate::access::{AccessRequest, Decision, Gate, Scope};
use crate::grants::{AlwaysGrants, GrantId, Hold};
use crate::log::log;

const PATIENCE: Duration = Duration::from_secs(120);

const NO_ONE_AVAILABLE: &str = "no one is available to approve";
const TIMED_OUT: &str = "approval timed out";
const UNSENDABLE: &str = "the approval request cannot be sent";
const REFUSED_BY_PERSON: &str = "the person denied the request";
const REQUESTER_GONE: &str = "the requester disconnected";

#[derive(Debug)]
pub struct Broker {
    grants: Grants,
    always: Option<AlwaysGrants>,
    apps: Arc<Apps>,
    waiting: Arc<Waiting>,
    next_id: AtomicU64,
    patience: Duration,
}

impl Broker {
    pub fn new() -> Self {
        Self {
            grants: Grants::default(),
            always: None,
            apps: Arc::new(Apps::default()),
            waiting: Arc::new(Waiting::default()),
            next_id: AtomicU64::new(1),
            patience: PATIENCE,
        }
    }

    pub fn with_always(always: AlwaysGrants) -> Self {
        Self {
            always: Some(always),
            ..Self::new()
        }
    }

    #[must_use]
    pub fn with_patience(self, patience: Duration) -> Self {
        Self { patience, ..self }
    }

    pub(crate) fn hold(&self, grant: GrantId, stop: &CancellationToken) -> Option<Hold> {
        let always = self.always.as_ref()?;

        Some(always.hold(grant, stop))
    }

    pub(crate) fn register(&self, outgoing: mpsc::Sender<Message>) -> Registration {
        self.apps.join(outgoing)
    }

    pub(crate) fn tell_apps(&self, notification: &Notification) -> usize {
        self.apps.broadcast(notification)
    }

    pub(crate) fn answer(&self, responder: &Actor, answer: ApprovalAnswer) -> Result<(), RpcError> {
        if *responder != Actor::Person {
            return Err(RpcError::new(
                ErrorCode::DENIED,
                "only the person can answer approvals",
            ));
        }

        if !self.waiting.answer(answer.id, answer.decision) {
            return Err(RpcError::new(
                ErrorCode::APPROVAL_UNKNOWN,
                format!("no approval {} is waiting for an answer", answer.id),
            ));
        }

        Ok(())
    }

    async fn ask(&self, request: &AccessRequest<'_>, grantable: bool) -> Decision {
        if grantable && let Some(standing) = self.standing(request) {
            return standing;
        }

        match self.consult(request).await {
            Ok(decision) => self.settle(decision, request, grantable).await,
            Err(reason) => Decision::deny(reason),
        }
    }

    fn standing(&self, request: &AccessRequest<'_>) -> Option<Decision> {
        if self.grants.covers(&Grant::covering(request)) {
            return Some(Decision::Approved(Scope::UntilRestart));
        }

        let always = self.always.as_ref()?;
        let grant = always.covering(request)?;

        Some(Decision::Granted(grant))
    }

    async fn consult(&self, request: &AccessRequest<'_>) -> Result<ApprovalDecision, &'static str> {
        if request.departed.is_cancelled() {
            return Err(REQUESTER_GONE);
        }

        let mut question = self.waiting.open(self.next_id());

        let approval = notice::approval_request(question.id(), request);
        let notification = approval.to_notification().map_err(|_| UNSENDABLE)?;

        if self.apps.broadcast(&notification) == 0 {
            return Err(NO_ONE_AVAILABLE);
        }

        let answered = tokio::select! {
            () = request.departed.cancelled() => return Err(REQUESTER_GONE),
            answered = time::timeout(self.patience, question.answer()) => answered,
        };

        answered.ok().flatten().ok_or(TIMED_OUT)
    }

    async fn settle(
        &self,
        decision: ApprovalDecision,
        request: &AccessRequest<'_>,
        grantable: bool,
    ) -> Decision {
        match decision {
            ApprovalDecision::Deny => Decision::deny(REFUSED_BY_PERSON),
            ApprovalDecision::AllowUntilRestart if grantable => {
                self.grants.give(Grant::covering(request));
                Decision::Approved(Scope::UntilRestart)
            }
            ApprovalDecision::AllowAlways if grantable => self.keep(request).await,
            ApprovalDecision::AllowOnce
            | ApprovalDecision::AllowUntilRestart
            | ApprovalDecision::AllowAlways => Decision::Approved(Scope::Once),
        }
    }

    async fn keep(&self, request: &AccessRequest<'_>) -> Decision {
        let Some(always) = &self.always else {
            self.grants.give(Grant::covering(request));
            return Decision::Approved(Scope::UntilRestart);
        };

        match always.give(request).await {
            Ok(grant) => Decision::Granted(grant),
            Err(error) => {
                log!("the grant could not be kept, so the call is approved once: {error}");
                Decision::Approved(Scope::Once)
            }
        }
    }

    fn next_id(&self) -> ApprovalId {
        ApprovalId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for Broker {
    fn default() -> Self {
        Self::new()
    }
}

impl Gate for Broker {
    fn settled(&self, request: &AccessRequest<'_>) -> Option<Decision> {
        match Rule::for_request(request) {
            Rule::Allow => Some(Decision::Allow),
            Rule::ApprovedWithRun => Some(Decision::Approved(Scope::Run)),
            Rule::Ask { grantable: true } => self.standing(request),
            Rule::Ask { grantable: false } => None,
        }
    }

    async fn decide(&self, request: &AccessRequest<'_>) -> Decision {
        match Rule::for_request(request) {
            Rule::Allow => Decision::Allow,
            Rule::ApprovedWithRun => Decision::Approved(Scope::Run),
            Rule::Ask { grantable } => self.ask(request, grantable).await,
        }
    }
}
