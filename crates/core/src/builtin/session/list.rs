use std::sync::Arc;

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::SessionInfo;
use crate::broker::requester;
use crate::terminal::{Terminal, Terminals};

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
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<SessionInfo>;

    fn summary(&self, output: &Vec<SessionInfo>) -> Value {
        json!({ "sessions": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<SessionInfo>, ActionError> {
        let mut sessions = Vec::new();

        for terminal in self.terminals.all() {
            sessions.push(describe(&terminal).await);
        }

        Ok(sessions)
    }
}

async fn describe(terminal: &Arc<Terminal>) -> SessionInfo {
    let origin = terminal.origin();
    let size = terminal.size().await;

    SessionInfo {
        session: terminal.id(),
        target: origin.target.as_str().to_owned(),
        host: terminal.host(),
        actor: requester(&origin.actor),
        risk: origin.risk,
        columns: size.columns,
        rows: size.rows,
        attached: terminal.attached().await,
        age_ms: u64::try_from(terminal.age().as_millis()).unwrap_or(u64::MAX),
    }
}
