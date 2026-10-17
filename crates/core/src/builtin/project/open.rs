use std::path::PathBuf;

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};

use crate::projects::{ProjectInfo, Projects};

#[derive(Debug, Clone)]
pub struct ProjectOpen {
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ProjectPathParams {
    pub path: PathBuf,
}

impl ProjectOpen {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ProjectOpen {
    const ID: &'static str = "project.open";
    const TITLE: &'static str = "Open a project folder and watch its resources";
    const RISK: Risk = Risk::Write;
    const PERSON_ONLY: bool = true;

    type Params = ProjectPathParams;
    type Output = ProjectInfo;

    fn summary(&self, output: &ProjectInfo) -> Value {
        json!({ "id": output.id, "slug": output.slug, "resources": output.resources })
    }

    async fn run(
        &self,
        _context: &Context,
        params: ProjectPathParams,
    ) -> Result<ProjectInfo, ActionError> {
        if !params.path.is_absolute() {
            return Err(ActionError::InvalidParams(format!(
                "path must be absolute, got {}",
                params.path.display()
            )));
        }

        let projects = self.projects.clone();

        blocking(move || projects.open(&params.path)).await
    }
}
