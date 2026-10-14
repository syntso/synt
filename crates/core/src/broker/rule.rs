use synt_actions::Risk;
use synt_journal::Actor;

use crate::access::AccessRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rule {
    Allow,
    ApprovedWithRun,
    Ask { grantable: bool },
}

impl Rule {
    pub(super) fn for_request(request: &AccessRequest<'_>) -> Self {
        let own = Self::for_call(request.actor, request.risk);

        let rule = match request.origin {
            Some(origin) => own.stricter(Self::for_call(origin, request.risk)),
            None => own,
        };

        let covered = request.run_approved && request.risk < Risk::Privileged;

        match rule {
            Self::Ask { .. } if covered => Self::ApprovedWithRun,
            rule => rule,
        }
    }

    pub(super) fn for_call(actor: &Actor, risk: Risk) -> Self {
        let personal = matches!(actor, Actor::Person | Actor::Core);

        match risk {
            Risk::Read => Self::Allow,
            Risk::Write if personal => Self::Allow,
            Risk::Write => Self::Ask { grantable: true },
            Risk::Privileged => Self::Ask {
                grantable: personal,
            },
        }
    }

    fn stricter(self, other: Self) -> Self {
        match (self, other) {
            (Self::Ask { grantable: own }, Self::Ask { grantable: origin }) => Self::Ask {
                grantable: own && origin,
            },
            (asked @ Self::Ask { .. }, _) | (_, asked @ Self::Ask { .. }) => asked,
            (Self::ApprovedWithRun, _) | (_, Self::ApprovedWithRun) => Self::ApprovedWithRun,
            (Self::Allow, Self::Allow) => Self::Allow,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use synt_actions::Risk;
    use synt_journal::{ActionId, Actor, Name};
    use tokio_util::sync::CancellationToken;

    use super::Rule;
    use crate::access::AccessRequest;

    fn rule(origin: Option<&Actor>, risk: Risk) -> Rule {
        let plugin = Actor::Plugin(Name::new("uptime").unwrap());

        judged(&plugin, origin, risk, false)
    }

    fn judged(actor: &Actor, origin: Option<&Actor>, risk: Risk, run_approved: bool) -> Rule {
        let action = ActionId::new("ssh.exec").unwrap();
        let departed = CancellationToken::new();

        Rule::for_request(&AccessRequest {
            actor,
            origin,
            action: &action,
            risk,
            target: None,
            destination: None,
            params: &Value::Null,
            run_approved,
            departed: &departed,
        })
    }

    #[test]
    fn a_nested_call_asks_as_strictly_as_its_origin() {
        let agent = Actor::Agent(Name::new("cursor").unwrap());

        assert_eq!(rule(Some(&Actor::Person), Risk::Read), Rule::Allow);
        assert_eq!(
            rule(Some(&Actor::Person), Risk::Write),
            Rule::Ask { grantable: true }
        );
        assert_eq!(
            rule(Some(&agent), Risk::Write),
            Rule::Ask { grantable: true }
        );
        assert_eq!(
            rule(Some(&agent), Risk::Privileged),
            Rule::Ask { grantable: false }
        );
    }

    #[test]
    fn an_origin_never_loosens_the_rule() {
        let agent = Actor::Agent(Name::new("cursor").unwrap());

        assert_eq!(rule(None, Risk::Write), Rule::Ask { grantable: true });
        assert_eq!(rule(Some(&agent), Risk::Read), Rule::Allow);
    }

    #[test]
    fn a_run_approved_up_front_covers_its_read_and_write_steps_only() {
        let runbook = Actor::Runbook(Name::new("@lab/deploy").unwrap());
        let person = Some(&Actor::Person);

        assert_eq!(judged(&runbook, person, Risk::Read, true), Rule::Allow);
        assert_eq!(
            judged(&runbook, person, Risk::Write, true),
            Rule::ApprovedWithRun
        );
        assert_eq!(
            judged(&runbook, person, Risk::Privileged, true),
            Rule::Ask { grantable: false }
        );
        assert_eq!(
            judged(&runbook, person, Risk::Write, false),
            Rule::Ask { grantable: true }
        );
    }
}
