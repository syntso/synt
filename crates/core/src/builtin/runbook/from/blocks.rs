use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_proto::OneLine;

use super::Drafted;
use crate::CoreError;
use crate::runbooks::{Draft, DraftStep, Runbooks};
use crate::terminals::{Block, SessionId};

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
    pub name: Option<OneLine>,
}

impl RunbookFromBlocks {
    pub fn new(runbooks: Runbooks) -> Self {
        Self { runbooks }
    }

    fn command_of(blocks: &[Block], number: u64) -> Result<String, CoreError> {
        let block = blocks
            .iter()
            .find(|block| block.number == number)
            .ok_or(CoreError::NoBlock(number))?;

        block
            .command
            .clone()
            .filter(|command| !command.trim().is_empty())
            .ok_or(CoreError::NoCommand(number))
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
            return Err(CoreError::Picks {
                what: "blocks",
                most: MOST_BLOCKS,
                numbers: "block numbers",
            }
            .into());
        }

        let session = params.session.to_string();
        let terminal = self
            .runbooks
            .terminals()
            .get(params.session)
            .ok_or_else(|| CoreError::NoSessionNamed(session.clone()))?;
        let host = terminal
            .origin()
            .target
            .clone()
            .ok_or(CoreError::LocalSession(session))?;

        let blocks = terminal.blocks().await;
        let name = match params.name {
            Some(name) => name,
            None => OneLine::new(DEFAULT_NAME).map_err(CoreError::NotOneLine)?,
        };
        let mut draft = Draft::new(name);

        for number in &params.blocks {
            let command = Self::command_of(&blocks, *number)?;

            draft.push(DraftStep::shell(host.as_str(), command));
        }

        Ok(Drafted::new(draft.build()?)?)
    }
}
