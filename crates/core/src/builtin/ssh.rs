mod connect_limit;
mod connections;
mod exec;
mod failure;
mod login;
mod privilege;
mod warm;

pub(crate) use connect_limit::within_connect_limit;
pub use connections::SshConnections;
pub use exec::SshExec;
pub(crate) use failure::failure;
pub(crate) use privilege::{escalates, is_root};
pub use warm::SshWarm;

use synt_actions::{ActionError, Registry};
use synt_ssh::Pool;

use crate::projects::Projects;

#[derive(Debug, Clone)]
pub struct SshActions {
    pool: Pool,
    projects: Projects,
}

impl SshActions {
    pub fn new(pool: Pool, projects: Projects) -> Self {
        Self { pool, projects }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(SshExec::new(self.pool.clone(), self.projects.clone()))?;
        registry.register(SshWarm::new(self.pool.clone(), self.projects.clone()))?;
        registry.register(SshConnections::new(self.pool.clone()))
    }
}
