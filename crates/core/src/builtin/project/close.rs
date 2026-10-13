use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::blocking::blocking;
use crate::projects::{ProjectInfo, Projects};

#[derive(Debug, Clone)]
pub struct ProjectClose {
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ProjectIdParams {
    pub id: String,
}

impl ProjectClose {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ProjectClose {
    const ID: &'static str = "project.close";
    const TITLE: &'static str = "Close a project and stop watching its folder";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = ProjectIdParams;
    type Output = ProjectInfo;

    fn summary(&self, output: &ProjectInfo) -> Value {
        json!({ "id": output.id, "slug": output.slug })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ProjectIdParams,
    ) -> Result<ProjectInfo, ActionError> {
        let projects = self.projects.clone();

        blocking(move || projects.close(&params.id))
            .await
            .map_err(|error| ActionError::Failed(error.to_string()))
    }
}
