mod get;
mod list;
mod params;
mod put;
mod remove;

pub use get::ResourceGet;
pub use list::ResourceList;
pub(crate) use params::ResourceParams;
pub use put::ResourcePut;
pub use remove::ResourceRemove;

use synt_actions::{ActionError, ActionGroup, Registry};

use crate::projects::Projects;

#[derive(Debug, Clone)]
pub(crate) struct ResourceActions {
    projects: Projects,
}

impl ResourceActions {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl ActionGroup for ResourceActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(ResourceList::new(self.projects.clone()))?;
        registry.register(ResourceGet::new(self.projects.clone()))?;
        registry.register(ResourcePut::new(self.projects.clone()))?;
        registry.register(ResourceRemove::new(self.projects.clone()))
    }
}
