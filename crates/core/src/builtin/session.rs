mod attach;
mod blocks;
mod close;
mod dimensions;
mod info;
mod integration;
mod list;
mod local_open;
mod open;
mod resize;
mod size;

pub use attach::SessionAttach;
pub use blocks::SessionBlocks;
pub use close::SessionClose;
use dimensions::Dimensions;
pub(crate) use info::SessionInfo;
pub use integration::SessionIntegration;
pub use list::SessionList;
pub use local_open::LocalSessionOpen;
pub(crate) use open::SessionOpenParams;
pub use open::SshSessionOpen;
pub use resize::SessionResize;
pub(crate) use size::SessionSize;

use synt_actions::{ActionError, ActionGroup, Registry};
use synt_ssh::Pool;

use crate::projects::Projects;
use crate::terminals::{AttachedSession, Terminals};

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
}

impl ActionGroup for SessionActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
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
