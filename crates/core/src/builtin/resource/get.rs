use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};

use super::ResourceParams;
use crate::projects::{Projects, ResourceEntry};

#[derive(Debug, Clone)]
pub struct ResourceGet {
    projects: Projects,
}

impl ResourceGet {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ResourceGet {
    const ID: &'static str = "resource.get";
    const TITLE: &'static str = "Show one resource of a project";
    const RISK: Risk = Risk::Read;

    type Params = ResourceParams;
    type Output = ResourceEntry;

    fn summary(&self, output: &ResourceEntry) -> Value {
        json!({ "name": output.name })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ResourceParams,
    ) -> Result<ResourceEntry, ActionError> {
        let projects = self.projects.clone();

        blocking(move || projects.resource(&params.project, &params.name)).await
    }
}
