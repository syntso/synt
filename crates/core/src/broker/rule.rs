use synt_actions::Risk;
use synt_journal::Actor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rule {
    Allow,
    Ask { grantable: bool },
}

impl Rule {
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
}
