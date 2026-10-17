use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::{ConnectionState, Pool};

use super::within_connect_limit;
use crate::projects::Projects;

#[derive(Debug, Clone)]
pub struct SshWarm {
    pool: Pool,
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warmed {
    pub target: String,
    pub state: ConnectionState,
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
        self.projects.login_risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, (): &()) -> Option<String> {
        self.projects.host_destination(target)
    }

    fn summary(&self, output: &Warmed) -> Value {
        json!({ "state": output.state })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<Warmed, ActionError> {
        let target = self.projects.reach_host(context).await?;

        let state = within_connect_limit(self.pool.warm(&target)).await?;

        Ok(Warmed {
            target: target.label(),
            state,
        })
    }
}
