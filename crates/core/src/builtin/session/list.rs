use std::sync::Arc;

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::SessionInfo;
use crate::broker::requester;
use crate::clock::milliseconds;
use crate::terminals::{Session, Terminals};

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
            sessions.push(describe(&session).await);
        }

        Ok(sessions)
    }
}

async fn describe(session: &Arc<Session>) -> SessionInfo {
    let origin = session.origin();
    let size = session.size().await;

    SessionInfo {
        session: session.id(),
        kind: session.kind(),
        target: origin
            .target
            .as_ref()
            .map(|target| target.as_str().to_owned()),
        host: session.host(),
        actor: requester(&origin.actor),
        risk: origin.risk,
        columns: size.columns,
        rows: size.rows,
        attached: session.attached().await,
        blocks: session.blocks().await.len(),
        age_ms: milliseconds(session.age()),
    }
}
