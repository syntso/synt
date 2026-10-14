use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

pub struct ExecStub;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct StubParams {
    pub command: String,

    #[serde(default)]
    pub privileged: bool,
}

impl ExecStub {
    pub const STDOUT: &'static str = "up 3 days";
}

impl Action for ExecStub {
    const ID: &'static str = "ssh.exec";
    const TITLE: &'static str = "Pretend to run a command over SSH";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = StubParams;
    type Output = Value;

    fn risk(&self, _target: Option<&ResourceRef>, params: &StubParams) -> Risk {
        if params.privileged {
            return Risk::Privileged;
        }

        Risk::Write
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &StubParams) -> Option<String> {
        target.map(|target| format!("{target} = root@203.0.113.7:22"))
    }

    async fn run(&self, _context: &Context, _params: StubParams) -> Result<Value, ActionError> {
        Ok(json!({ "exit": 0, "stdout": Self::STDOUT }))
    }
}
