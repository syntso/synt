use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::builtin::blocking::blocking;
use crate::projects::{Projects, ResourceListing};

#[derive(Debug, Clone)]
pub struct ResourceList {
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ResourceListParams {
    pub project: String,
}

impl ResourceList {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ResourceList {
    const ID: &'static str = "resource.list";
    const TITLE: &'static str = "List the resources of a project";
    const RISK: Risk = Risk::Read;

    type Params = ResourceListParams;
    type Output = ResourceListing;

    fn summary(&self, output: &ResourceListing) -> Value {
        json!({
            "project": output.project,
            "resources": output.resources.len(),
            "problems": output.problems.len(),
        })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ResourceListParams,
    ) -> Result<ResourceListing, ActionError> {
        let projects = self.projects.clone();

        blocking(move || projects.listing(&params.project)).await
    }
}
