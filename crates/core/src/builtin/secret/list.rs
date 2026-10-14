use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::builtin::blocking::blocking;
use crate::secrets::{SecretInfo, Secrets};

#[derive(Clone)]
pub struct SecretList {
    secrets: Secrets,
}

impl SecretList {
    pub fn new(secrets: Secrets) -> Self {
        Self { secrets }
    }
}

impl Action for SecretList {
    const ID: &'static str = "secret.list";
    const TITLE: &'static str = "List the names of the secrets the core keeps";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<SecretInfo>;

    fn summary(&self, output: &Vec<SecretInfo>) -> Value {
        json!({ "secrets": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<SecretInfo>, ActionError> {
        let secrets = self.secrets.clone();

        blocking(move || secrets.list()).await
    }
}
