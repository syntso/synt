use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::terminals::{Block, SessionId, Terminals};

#[derive(Debug, Clone)]
pub struct SessionBlocks {
    terminals: Terminals,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct SessionBlocksParams {
    pub session: SessionId,
}

impl SessionBlocks {
    pub fn new(terminals: Terminals) -> Self {
        Self { terminals }
    }
}

impl Action for SessionBlocks {
    const ID: &'static str = "session.blocks";
    const TITLE: &'static str = "List the finished commands of a terminal session";
    const RISK: Risk = Risk::Read;
    const PERSON_ONLY: bool = true;

    type Params = SessionBlocksParams;
    type Output = Vec<Block>;

    fn summary(&self, output: &Vec<Block>) -> Value {
        json!({ "blocks": output.len() })
    }

    async fn run(
        &self,
        _context: &Context,
        params: SessionBlocksParams,
    ) -> Result<Vec<Block>, ActionError> {
        let session = self.terminals.find(params.session)?;

        Ok(session.blocks().await)
    }
}
