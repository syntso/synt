use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};

use crate::ssh_keys::{KeyInfo, SshKeys};

#[derive(Clone)]
pub struct KeyList {
    keys: SshKeys,
}

impl KeyList {
    pub fn new(keys: SshKeys) -> Self {
        Self { keys }
    }
}

impl Action for KeyList {
    const ID: &'static str = "key.list";
    const TITLE: &'static str = "List the SSH keys the core keeps, with their public keys";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<KeyInfo>;

    fn summary(&self, output: &Vec<KeyInfo>) -> Value {
        json!({ "keys": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<KeyInfo>, ActionError> {
        let keys = self.keys.clone();

        blocking(move || keys.list()).await
    }
}
