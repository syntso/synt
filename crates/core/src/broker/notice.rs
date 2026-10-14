use synt_actions::Risk;
use synt_journal::Actor;
use synt_proto::{Requester, RiskLevel};

pub(crate) fn requester(actor: &Actor) -> Requester {
    match actor {
        Actor::Person => Requester::Person,
        Actor::Core => Requester::Core,
        Actor::Agent(name) => Requester::Agent(name.as_str().to_owned()),
        Actor::Runbook(name) => Requester::Runbook(name.as_str().to_owned()),
        Actor::Plugin(name) => Requester::Plugin(name.as_str().to_owned()),
    }
}

pub(crate) fn risk_level(risk: Risk) -> RiskLevel {
    match risk {
        Risk::Read => RiskLevel::Read,
        Risk::Write => RiskLevel::Write,
        Risk::Privileged => RiskLevel::Privileged,
    }
}
