use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_proto::EndReason;

use crate::terminals::{Ending, SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct SessionClose {
    terminals: Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionCloseParams {
    pub session: SessionId,
}

impl SessionClose {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for SessionClose {
    const ID: &'static str = "session.close";
    const TITLE: &'static str = "Close a terminal session";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SessionCloseParams;
    type Output = Ending;

    fn summary(&self, output: &Ending) -> Value {
        serde_json::to_value(output).unwrap_or_default()
    }

    async fn run(
        &self,
        _context: &Context,
        params: SessionCloseParams,
    ) -> Result<Ending, ActionError> {
        let session = self.terminals.find(params.session)?;

        session
            .close(EndReason::Closed)
            .await
            .ok_or_else(|| ActionError::failed("the session did not end"))
    }
}
