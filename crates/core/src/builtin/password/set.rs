use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::builtin::blocking::blocking;
use crate::passwords::{PasswordInfo, PasswordName, Passwords};

const PASSWORD_FIELD: &str = "password";
const LONGEST_PASSWORD: usize = 4096;

#[derive(Clone)]
pub struct PasswordSet {
    passwords: Passwords,
}

#[derive(Deserialize, JsonSchema, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub struct SetParams {
    pub name: String,
    pub password: String,
}

impl PasswordSet {
    pub fn new(passwords: Passwords) -> Self {
        Self { passwords }
    }
}

impl Action for PasswordSet {
    const ID: &'static str = "password.set";
    const TITLE: &'static str = "Keep a password in the core's vault for resources to name";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = SetParams;
    type Output = PasswordInfo;

    fn check(&self, _target: Option<&ResourceRef>, params: &SetParams) -> Result<(), ActionError> {
        PasswordName::parse(&params.name).map_err(ActionError::InvalidParams)?;

        if params.password.is_empty() || params.password.len() > LONGEST_PASSWORD {
            return Err(ActionError::InvalidParams(format!(
                "a password must be 1 to {LONGEST_PASSWORD} bytes"
            )));
        }

        Ok(())
    }

    fn recorded_params(&self, params: &Value) -> Value {
        let mut recorded = params.clone();

        if let Some(fields) = recorded.as_object_mut() {
            fields.remove(PASSWORD_FIELD);
        }

        recorded
    }

    fn summary(&self, output: &PasswordInfo) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: SetParams,
    ) -> Result<PasswordInfo, ActionError> {
        let name = PasswordName::parse(&params.name).map_err(ActionError::InvalidParams)?;
        let passwords = self.passwords.clone();

        blocking(move || passwords.store(&name, &params.password)).await
    }
}
