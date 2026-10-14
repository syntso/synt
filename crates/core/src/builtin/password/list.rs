use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::builtin::blocking::blocking;
use crate::passwords::{PasswordInfo, Passwords};

#[derive(Clone)]
pub struct PasswordList {
    passwords: Passwords,
}

impl PasswordList {
    pub fn new(passwords: Passwords) -> Self {
        Self { passwords }
    }
}

impl Action for PasswordList {
    const ID: &'static str = "password.list";
    const TITLE: &'static str = "List the names of the passwords the core keeps";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<PasswordInfo>;

    fn summary(&self, output: &Vec<PasswordInfo>) -> Value {
        json!({ "passwords": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<PasswordInfo>, ActionError> {
        let passwords = self.passwords.clone();

        blocking(move || passwords.list()).await
    }
}
