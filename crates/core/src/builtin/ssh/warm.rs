use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_ssh::Pool;

use super::{destination, login_risk, reach_host, within_connect_limit};
use crate::projects::Projects;

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
        login_risk(&self.projects, target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, (): &()) -> Option<String> {
        destination(&self.projects, target)
    }

    fn summary(&self, output: &Warmed) -> Value {
        json!({ "state": output.state })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<Warmed, ActionError> {
        let target = reach_host(&self.projects, context).await?;

        let state = within_connect_limit(self.pool.warm(&target)).await?;

        Ok(Warmed {
            target: target.label(),
            state: state.name().to_owned(),
        })
    }
}
