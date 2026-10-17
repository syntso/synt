use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_ssh::Pool;

use crate::CoreError;
use crate::blocking::blocking;
use crate::ssh_keys::{KeyInfo, KeyName, SshKeys};

#[derive(Clone)]
pub struct KeyRemove {
    keys: SshKeys,
    pool: Pool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveParams {
    pub name: KeyName,
}

impl KeyRemove {
    pub fn new(keys: SshKeys, pool: Pool) -> Self {
        Self { keys, pool }
    }
}

impl Action for KeyRemove {
    const ID: &'static str = "key.remove";
    const TITLE: &'static str = "Delete an SSH key from the core's vault";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = RemoveParams;
    type Output = KeyInfo;

    fn summary(&self, output: &KeyInfo) -> Value {
        output.summary()
    }

    async fn run(&self, _context: &Context, params: RemoveParams) -> Result<KeyInfo, ActionError> {
        let keys = self.keys.clone();

        let removed = blocking::<_, CoreError>(move || keys.remove(&params.name)).await?;

        self.pool.forget_vault_key(removed.name.as_str());

        Ok(removed)
    }
}
