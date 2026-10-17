mod close;
mod list;
mod open;

pub use close::ProjectClose;
pub use list::ProjectList;
pub use open::ProjectOpen;

use synt_actions::{ActionError, ActionGroup, Registry};

use crate::projects::Projects;

#[derive(Debug, Clone)]
pub(crate) struct ProjectActions {
    projects: Projects,
}

impl ProjectActions {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl ActionGroup for ProjectActions {
    fn register(&self, registry: &mut Registry) -> Result<(), ActionError> {
        registry.register(ProjectOpen::new(self.projects.clone()))?;
        registry.register(ProjectClose::new(self.projects.clone()))?;
        registry.register(ProjectList::new(self.projects.clone()))
    }
}
