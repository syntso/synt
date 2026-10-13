use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::AttachedSession;
use super::checks::{connection, find};
use crate::terminal::{SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct SessionAttach {
    terminals: Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionAttachParams {
    pub session: SessionId,
}

impl SessionAttach {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for SessionAttach {
    const ID: &'static str = "session.attach";
    const TITLE: &'static str = "Attach to an open terminal session";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SessionAttachParams;
    type Output = AttachedSession;

    fn risk(&self, _target: Option<&ResourceRef>, params: &SessionAttachParams) -> Risk {
        self.terminals
            .get(params.session)
            .map_or(Risk::Write, |terminal| terminal.origin().risk)
    }

    fn summary(&self, output: &AttachedSession) -> Value {
        json!({
            "session": output.session,
            "stream": output.stream,
            "replayed_bytes": output.replayed_bytes,
        })
    }

    async fn run(
        &self,
        context: &Context,
        params: SessionAttachParams,
    ) -> Result<AttachedSession, ActionError> {
        let link = connection(context)?;
        let terminal = find(&self.terminals, params.session)?;
        let attached = terminal.attach(&link).await;

        Ok(AttachedSession::new(params.session, &attached))
    }
}
