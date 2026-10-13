use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};

use crate::blocking::blocking;
use crate::projects::{ProjectInfo, Projects};

#[derive(Debug, Clone)]
pub struct ProjectList {
    projects: Projects,
}

impl ProjectList {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ProjectList {
    const ID: &'static str = "project.list";
    const TITLE: &'static str = "List the open projects";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<ProjectInfo>;

    fn summary(&self, output: &Vec<ProjectInfo>) -> Value {
        json!({ "projects": output.len() })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<ProjectInfo>, ActionError> {
        let projects = self.projects.clone();

        blocking(move || projects.list())
            .await
            .map_err(|error| ActionError::Failed(error.to_string()))
    }
}
