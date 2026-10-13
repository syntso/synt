use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use synt_journal::Actor;
use synt_proto::{ApprovalAnswer, ApprovalDecision, ApprovalId, ErrorCode, Message, RpcError};
use tokio::sync::mpsc;
use tokio::time;

use super::apps::Apps;
use super::grant::Grant;
use super::grants::Grants;
use super::notice;
use super::registration::Registration;
use super::rule::Rule;
use super::waiting::Waiting;
use crate::access::{AccessRequest, Decision, Gate, Scope};

const PATIENCE: Duration = Duration::from_secs(120);

const NO_ONE_AVAILABLE: &str = "no one is available to approve";
const TIMED_OUT: &str = "approval timed out";
const UNSENDABLE: &str = "the approval request cannot be sent";
const REFUSED_BY_PERSON: &str = "the person denied the request";
const REQUESTER_GONE: &str = "the requester disconnected";

#[derive(Debug)]
pub struct Broker {
    grants: Grants,
    apps: Arc<Apps>,
    waiting: Arc<Waiting>,
    next_id: AtomicU64,
}

impl Broker {
    pub fn new() -> Self {
        Self {
            grants: Grants::default(),
            apps: Arc::new(Apps::default()),
            waiting: Arc::new(Waiting::default()),
            next_id: AtomicU64::new(1),
        }
    }

    pub(crate) fn register(&self, outgoing: mpsc::Sender<Message>) -> Registration {
        self.apps.join(outgoing)
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
        let grant = Grant::covering(request);

        if grantable && self.grants.covers(&grant) {
            return Decision::Approved(Scope::UntilRestart);
        }

        match self.consult(request).await {
            Ok(decision) => self.settle(decision, grant, grantable),
            Err(reason) => Decision::deny(reason),
        }
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
            answered = time::timeout(PATIENCE, question.answer()) => answered,
        };

        answered.ok().flatten().ok_or(TIMED_OUT)
    }

    fn settle(&self, decision: ApprovalDecision, grant: Grant, grantable: bool) -> Decision {
        match decision {
            ApprovalDecision::Deny => Decision::deny(REFUSED_BY_PERSON),
            ApprovalDecision::AllowUntilRestart if grantable => {
                self.grants.give(grant);
                Decision::Approved(Scope::UntilRestart)
            }
            ApprovalDecision::AllowOnce | ApprovalDecision::AllowUntilRestart => {
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
        match Rule::for_call(request.actor, request.risk) {
            Rule::Allow => Some(Decision::Allow),
            Rule::Ask { grantable: true } if self.grants.covers(&Grant::covering(request)) => {
                Some(Decision::Approved(Scope::UntilRestart))
            }
            Rule::Ask { .. } => None,
        }
    }

    async fn decide(&self, request: &AccessRequest<'_>) -> Decision {
        match Rule::for_call(request.actor, request.risk) {
            Rule::Allow => Decision::Allow,
            Rule::Ask { grantable } => self.ask(request, grantable).await,
        }
    }
}
