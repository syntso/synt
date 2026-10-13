use synt_actions::Risk;
use synt_journal::Actor;
use synt_proto::{ApprovalId, ApprovalRequest, Requester, RiskLevel};

use crate::access::AccessRequest;

pub(super) fn approval_request(id: ApprovalId, request: &AccessRequest<'_>) -> ApprovalRequest {
    ApprovalRequest {
        id,
        actor: requester(request.actor),
        action: request.action.as_str().to_owned(),
        target: request.target.map(|target| target.as_str().to_owned()),
        risk: risk_level(request.risk),
        params: request.params.clone(),
    }
}

fn requester(actor: &Actor) -> Requester {
    match actor {
        Actor::Person => Requester::Person,
        Actor::Core => Requester::Core,
        Actor::Agent(name) => Requester::Agent(name.as_str().to_owned()),
        Actor::Runbook(name) => Requester::Runbook(name.as_str().to_owned()),
        Actor::Plugin(name) => Requester::Plugin(name.as_str().to_owned()),
    }
}

fn risk_level(risk: Risk) -> RiskLevel {
    match risk {
        Risk::Read => RiskLevel::Read,
        Risk::Write => RiskLevel::Write,
        Risk::Privileged => RiskLevel::Privileged,
    }
}
