use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::PasswordNameParams;
use crate::builtin::blocking::blocking;
use crate::passwords::{PasswordInfo, Passwords};

#[derive(Clone)]
pub struct PasswordRemove {
    passwords: Passwords,
}

impl PasswordRemove {
    pub fn new(passwords: Passwords) -> Self {
        Self { passwords }
    }
}

impl Action for PasswordRemove {
    const ID: &'static str = "password.remove";
    const TITLE: &'static str = "Delete a password from the core's vault";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = PasswordNameParams;
    type Output = PasswordInfo;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &PasswordNameParams,
    ) -> Result<(), ActionError> {
        params.password_name().map(|_| ())
    }

    fn summary(&self, output: &PasswordInfo) -> Value {
        output.summary()
    }

    async fn run(
        &self,
        _context: &Context,
        params: PasswordNameParams,
    ) -> Result<PasswordInfo, ActionError> {
        let name = params.password_name()?;
        let passwords = self.passwords.clone();

        blocking(move || passwords.remove(&name)).await
    }
}
