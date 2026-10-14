use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::secret_name_params::SecretNameParams;
use crate::builtin::blocking::blocking;
use crate::secrets::{SecretInfo, Secrets};

#[derive(Clone)]
pub struct SecretRemove {
    secrets: Secrets,
}

impl SecretRemove {
    pub fn new(secrets: Secrets) -> Self {
        Self { secrets }
    }
}

impl Action for SecretRemove {
    const ID: &'static str = "secret.remove";
    const TITLE: &'static str = "Delete a secret from the core's vault";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SecretNameParams;
    type Output = SecretInfo;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &SecretNameParams,
    ) -> Result<(), ActionError> {
        params.secret_name().map(drop)
    }

    fn summary(&self, output: &SecretInfo) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: SecretNameParams,
    ) -> Result<SecretInfo, ActionError> {
        let name = params.secret_name()?;
        let secrets = self.secrets.clone();

        blocking(move || secrets.remove(&name)).await
    }
}
