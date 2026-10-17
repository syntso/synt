use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};
use synt_platform::SecretName;
use zeroize::Zeroizing;

use crate::secrets::{SecretInfo, Secrets};

#[derive(Clone)]
pub struct SecretPut {
    secrets: Secrets,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PutParams {
    pub name: SecretName,
    pub value: String,
}

impl SecretPut {
    pub fn new(secrets: Secrets) -> Self {
        Self { secrets }
    }
}

impl Action for SecretPut {
    const ID: &'static str = "secret.put";
    const TITLE: &'static str =
        "Keep a secret in the core's vault, for database passwords and extensions";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = PutParams;
    type Output = SecretInfo;

    fn recorded_params(&self, params: &Value) -> Value {
        let value_bytes = params.get("value").and_then(Value::as_str).map(str::len);

        json!({ "name": params.get("name"), "value_bytes": value_bytes })
    }

    fn summary(&self, output: &SecretInfo) -> Value {
        output.summary()
    }

    async fn run(&self, _context: &Context, params: PutParams) -> Result<SecretInfo, ActionError> {
        let value = Zeroizing::new(params.value);
        let secrets = self.secrets.clone();

        blocking(move || secrets.put(&params.name, value.as_bytes())).await
    }
}
