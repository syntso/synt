use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};

use super::Drafted;
use crate::runbooks::{Draft, DraftStep, Runbooks};
use crate::terminal::{Block, SessionId};

const MOST_BLOCKS: usize = 100;
const DEFAULT_NAME: &str = "Steps from a terminal session";

#[derive(Debug, Clone)]
pub struct RunbookFromBlocks {
    runbooks: Runbooks,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FromBlocksParams {
    pub session: SessionId,
    pub blocks: Vec<u64>,

    #[serde(default)]
    pub name: Option<String>,
}

impl RunbookFromBlocks {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }
}

impl Action for RunbookFromBlocks {
    const ID: &'static str = "runbook.from_blocks";
    const TITLE: &'static str = "Draft a runbook from commands of a terminal session";
    const RISK: Risk = Risk::Read;
    const PERSON_ONLY: bool = true;

    type Params = FromBlocksParams;
    type Output = Drafted;

    fn summary(&self, output: &Drafted) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: FromBlocksParams,
    ) -> Result<Drafted, ActionError> {
        if params.blocks.is_empty() || params.blocks.len() > MOST_BLOCKS {
            return Err(ActionError::InvalidParams(format!(
                "blocks must list 1 to {MOST_BLOCKS} block numbers"
            )));
        }

        let terminal = self
            .runbooks
            .terminals()
            .get(params.session)
            .ok_or_else(|| {
                ActionError::failed(format!("there is no open session {}", params.session))
            })?;

        let host = terminal.origin().target.clone().ok_or_else(|| {
            ActionError::InvalidParams(format!(
                "session {} is a local shell, so its commands have no host to run on",
                params.session
            ))
        })?;

        let blocks = terminal.blocks().await;
        let mut draft = Draft::new(params.name.unwrap_or_else(|| DEFAULT_NAME.to_owned()));

        for number in &params.blocks {
            let command = command_of(&blocks, *number)?;

            draft.push(DraftStep::shell(host.as_str(), command));
        }

        let runbook = draft.build().map_err(ActionError::InvalidParams)?;

        Drafted::new(runbook)
    }
}

fn command_of(blocks: &[Block], number: u64) -> Result<String, ActionError> {
    let block = blocks
        .iter()
        .find(|block| block.number == number)
        .ok_or_else(|| {
            ActionError::InvalidParams(format!("the session has no finished block {number}"))
        })?;

    block
        .command
        .clone()
        .filter(|command| !command.trim().is_empty())
        .ok_or_else(|| {
            ActionError::InvalidParams(format!(
                "block {number} has no command line; its shell did not report one"
            ))
        })
}
