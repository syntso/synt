use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Pool;
use tokio::time;

use super::{failure, hidden_root, is_root, logs_in_as_root, resolve};

const CONNECT_LIMIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct SshWarm {
    pool: Pool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warmed {
    pub target: String,
    pub state: String,
}

impl SshWarm {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

impl Action for SshWarm {
    const ID: &'static str = "ssh.warm";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Warmed;

    fn risk(&self, target: Option<&ResourceRef>, (): &()) -> Risk {
        if logs_in_as_root(target) {
            return Risk::Privileged;
        }

        Risk::Read
    }

    fn summary(&self, output: &Warmed) -> Value {
        json!({ "state": output.state })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<Warmed, ActionError> {
        let host = context.target().ok_or(ActionError::TargetRequired)?;
        let target = resolve(host.as_str()).await?;

        if is_root(target.user()) && context.risk() != Risk::Privileged {
            return Err(hidden_root(host));
        }

        let state = time::timeout(CONNECT_LIMIT, self.pool.warm(&target))
            .await
            .map_err(|_| {
                ActionError::failed(format!(
                    "could not connect within {} s",
                    CONNECT_LIMIT.as_secs()
                ))
            })?
            .map_err(|error| failure(&error))?;

        Ok(Warmed {
            target: target.label(),
            state: state.name().to_owned(),
        })
    }
}
