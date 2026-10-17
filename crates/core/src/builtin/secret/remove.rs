use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk, blocking};
use synt_platform::SecretName;

use crate::secrets::{SecretInfo, Secrets};

#[derive(Clone)]
pub struct SecretRemove {
    secrets: Secrets,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RemoveParams {
    pub name: SecretName,
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

    type Params = RemoveParams;
    type Output = SecretInfo;

    fn summary(&self, output: &SecretInfo) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: RemoveParams,
    ) -> Result<SecretInfo, ActionError> {
        let secrets = self.secrets.clone();

        blocking(move || secrets.remove(&params.name)).await
    }
}
