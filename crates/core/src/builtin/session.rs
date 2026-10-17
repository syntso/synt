mod attach;
mod blocks;
mod checks;
mod close;
mod info;
mod integration;
mod list;
mod local_open;
mod open;
mod resize;
mod revocation;
mod size;

pub(crate) use crate::terminals::AttachedSession;
pub use attach::SessionAttach;
pub use blocks::SessionBlocks;
pub use close::SessionClose;
pub(crate) use info::SessionInfo;
pub use integration::SessionIntegration;
pub use list::SessionList;
pub use local_open::LocalSessionOpen;
pub(crate) use open::SessionOpenParams;
pub use open::SshSessionOpen;
pub use resize::SessionResize;
pub(crate) use size::SessionSize;

use synt_actions::{ActionError, Registry};
use synt_ssh::Pool;

use crate::projects::Projects;
use crate::terminals::Terminals;

#[derive(Debug, Clone)]
pub struct SessionActions {
    pool: Pool,
    projects: Projects,
    terminals: Terminals,
}

impl SessionActions {
    pub fn new(pool: Pool, projects: Projects, terminals: Terminals) -> Self {
        Self {
            pool,
            projects,
            terminals,
        }
    }

    pub fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        let terminals = &self.terminals;

        registry.register(SshSessionOpen::new(
            self.pool.clone(),
            terminals.clone(),
            self.projects.clone(),
        ))?;
        registry.register(LocalSessionOpen::new(terminals.clone()))?;
        registry.register(SessionAttach::new(terminals.clone()))?;
        registry.register(SessionResize::new(terminals.clone()))?;
        registry.register(SessionClose::new(terminals.clone()))?;
        registry.register(SessionList::new(terminals.clone()))?;
        registry.register(SessionBlocks::new(terminals.clone()))?;
        registry.register(SessionIntegration)
    }
}
