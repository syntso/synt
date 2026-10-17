mod apps;
mod open_question;
mod registration;
mod rule;
mod unanswered;
mod until_restart;
mod waiting;

pub(crate) use registration::Registration;

use apps::Apps;
use open_question::OpenQuestion;
use rule::Rule;
use unanswered::Unanswered;
use until_restart::UntilRestart;
use waiting::Waiting;

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

use crate::access::{AccessRequest, Decision, Gate, Scope};
use crate::grants::{AlwaysGrants, GrantId, GrantScope, Hold};
use crate::log::log;

const PATIENCE: Duration = Duration::from_secs(120);

const REFUSED_BY_PERSON: &str = "the person denied the request";

#[derive(Debug)]
pub struct Broker {
    until_restart: UntilRestart,
    always: Option<AlwaysGrants>,
    apps: Arc<Apps>,
    waiting: Arc<Waiting>,
    next_id: AtomicU64,
    patience: Duration,
}

impl Broker {
    pub fn new() -> Self {
        Self {
            until_restart: UntilRestart::default(),
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
            Err(unanswered) => Decision::deny(unanswered.to_string()),
        }
    }

    fn standing(&self, request: &AccessRequest<'_>) -> Option<Decision> {
        if self.until_restart.covers(request) {
            return Some(Decision::Approved(Scope::UntilRestart));
        }

        let always = self.always.as_ref()?;
        let grant = always.covering(request)?;

        Some(Decision::Granted(grant))
    }

    async fn consult(&self, request: &AccessRequest<'_>) -> Result<ApprovalDecision, Unanswered> {
        if request.departed.is_cancelled() {
            return Err(Unanswered::RequesterGone);
        }

        let mut question = self.waiting.open(self.next_id());

        let approval = request.approval(question.id());
        let notification = approval
            .to_notification()
            .map_err(|_| Unanswered::Unsendable)?;

        if self.apps.broadcast(&notification) == 0 {
            return Err(Unanswered::NoOneAvailable);
        }

        let answered = tokio::select! {
            () = request.departed.cancelled() => return Err(Unanswered::RequesterGone),
            answered = time::timeout(self.patience, question.answer()) => answered,
        };

        answered.ok().flatten().ok_or(Unanswered::TimedOut)
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
                self.until_restart.give(GrantScope::covering(request));
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
            self.until_restart.give(GrantScope::covering(request));
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
        ApprovalId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
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
