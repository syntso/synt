use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use super::SessionSize;
use super::checks::{find, terminal_size};
use crate::terminal::{SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct SessionResize {
    terminals: Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
        let terminal = find(&self.terminals, params.session)?;

        terminal.resize(size).await.map_err(ActionError::Failed)?;

        Ok(SessionSize {
            session: params.session,
            columns: size.columns,
            rows: size.rows,
        })
    }
}
