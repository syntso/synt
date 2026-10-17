mod close;
mod info;
mod list;
mod open;

pub use close::TunnelClose;
pub(crate) use info::TunnelInfo;
pub(crate) use list::TunnelList;
pub use open::TunnelOpen;

use synt_actions::{ActionError, ActionGroup, Registry};
use synt_ssh::Tunnels;

use crate::projects::Projects;

#[derive(Debug, Clone)]
pub(crate) struct TunnelActions {
    tunnels: Tunnels,
    projects: Projects,
}

impl TunnelActions {
    pub fn new(tunnels: Tunnels, projects: Projects) -> Self {
        Self { tunnels, projects }
    }
}

impl ActionGroup for TunnelActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(TunnelOpen::new(self.tunnels.clone(), self.projects.clone()))?;
        registry.register(TunnelList::new(self.tunnels.clone()))?;
        registry.register(TunnelClose::new(self.tunnels.clone()))
    }
}
