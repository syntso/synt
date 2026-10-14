use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Pool;

use super::KeyNameParams;
use crate::blocking::blocking;
use crate::ssh_keys::{KeyInfo, SshKeys};

#[derive(Clone)]
pub struct KeyRemove {
    keys: SshKeys,
    pool: Pool,
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

    type Params = KeyNameParams;
    type Output = KeyInfo;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &KeyNameParams,
    ) -> Result<(), ActionError> {
        params.key_name().map(|_| ())
    }

    fn summary(&self, output: &KeyInfo) -> Value {
        json!({ "name": output.name, "kind": output.kind, "fingerprint": output.fingerprint })
    }

    async fn run(&self, _context: &Context, params: KeyNameParams) -> Result<KeyInfo, ActionError> {
        let name = params.key_name()?;
        let keys = self.keys.clone();

        let removed = blocking(move || keys.remove(&name))
            .await
            .map_err(|error| ActionError::Failed(error.to_string()))?;

        self.pool.forget_vault_key(&removed.name);

        Ok(removed)
    }
}
