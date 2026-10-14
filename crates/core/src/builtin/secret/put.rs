use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use zeroize::Zeroizing;

use super::put_params::PutParams;
use crate::blocking::blocking;
use crate::secrets::{SecretInfo, SecretName, Secrets};

#[derive(Clone)]
pub struct SecretPut {
    secrets: Secrets,
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

    fn check(&self, _target: Option<&ResourceRef>, params: &PutParams) -> Result<(), ActionError> {
        SecretName::parse(&params.name)
            .map(drop)
            .map_err(ActionError::InvalidParams)
    }

    fn recorded_params(&self, params: &Value) -> Value {
        let value_bytes = params.get("value").and_then(Value::as_str).map(str::len);

        json!({ "name": params.get("name"), "value_bytes": value_bytes })
    }

    fn summary(&self, output: &SecretInfo) -> Value {
        output.summary()
    }

    async fn run(&self, _context: &Context, params: PutParams) -> Result<SecretInfo, ActionError> {
        let name = SecretName::parse(&params.name).map_err(ActionError::InvalidParams)?;
        let value = Zeroizing::new(params.value);
        let secrets = self.secrets.clone();

        blocking(move || secrets.put(&name, value.as_bytes())).await
    }
}
