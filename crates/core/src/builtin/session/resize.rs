use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::SessionSize;
use super::checks::{find, terminal_size};
use crate::terminals::{SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct SessionResize {
    terminals: Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionResizeParams {
    pub session: SessionId,
    #[serde(rename = "cols")]
    pub columns: u16,
    pub rows: u16,
}

impl SessionResize {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for SessionResize {
    const ID: &'static str = "session.resize";
    const TITLE: &'static str = "Resize a terminal session";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SessionResizeParams;
    type Output = SessionSize;

    fn summary(&self, output: &SessionSize) -> Value {
        json!({ "cols": output.columns, "rows": output.rows })
    }

    async fn run(
        &self,
        _context: &Context,
        params: SessionResizeParams,
    ) -> Result<SessionSize, ActionError> {
        let size = terminal_size(params.columns, params.rows)?;
        let session = find(&self.terminals, params.session)?;

        session.resize(size).await?;

        Ok(SessionSize {
            session: params.session,
            columns: size.columns,
            rows: size.rows,
        })
    }
}
