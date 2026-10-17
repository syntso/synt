use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::SessionInfo;
use crate::terminals::Terminals;

#[derive(Debug, Clone)]
pub struct SessionList {
    terminals: Terminals,
}

impl SessionList {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for SessionList {
    const ID: &'static str = "session.list";
    const TITLE: &'static str = "List the open terminal sessions";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<SessionInfo>;

    fn summary(&self, output: &Vec<SessionInfo>) -> Value {
        json!({ "sessions": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<SessionInfo>, ActionError> {
        let mut sessions = Vec::new();

        for session in self.terminals.all() {
            sessions.push(SessionInfo::of(&session).await);
        }

        Ok(sessions)
    }
}
