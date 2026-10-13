use std::time::Duration;

use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Pool;
use tokio::time;

use super::{destination, failure, logs_in_as_root, reach_host};
use crate::projects::Projects;

const CONNECT_LIMIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct SshWarm {
    pool: Pool,
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warmed {
    pub target: String,
    pub state: String,
}

impl SshWarm {
    pub fn new(pool: Pool, projects: Projects) -> Self {
        Self { pool, projects }
    }
}

impl Action for SshWarm {
    const ID: &'static str = "ssh.warm";
    const TITLE: &'static str = "Connect to a host ahead of time";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = ();
    type Output = Warmed;

    fn risk(&self, target: Option<&ResourceRef>, (): &()) -> Risk {
        if logs_in_as_root(&self.projects, target) {
            return Risk::Privileged;
        }

        Risk::Read
    }

    fn destination(&self, target: Option<&ResourceRef>, (): &()) -> Option<String> {
        destination(&self.projects, target)
    }

    fn summary(&self, output: &Warmed) -> Value {
        json!({ "state": output.state })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<Warmed, ActionError> {
        let target = reach_host(&self.projects, context).await?;

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
