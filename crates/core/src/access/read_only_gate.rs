use synt_actions::{Handle, Risk};
use synt_journal::Actor;

use super::{Decision, Gate};

#[derive(Debug, Clone, Copy, Default)]
pub struct ReadOnlyGate;

impl Gate for ReadOnlyGate {
    async fn decide(&self, _actor: &Actor, action: &Handle) -> Decision {
        match action.risk() {
            Risk::Read => Decision::Allow,
            Risk::Write | Risk::Privileged => {
                Decision::deny("only read actions are allowed until approvals are available")
            }
        }
    }
}
