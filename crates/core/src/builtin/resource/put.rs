use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::blocking::blocking;
use crate::projects::{Projects, Resource, ResourceEntry};

#[derive(Debug, Clone)]
pub struct ResourcePut {
    projects: Projects,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub struct ResourcePutParams {
    #[serde(flatten)]
    pub resource: Resource,
}

impl ResourcePut {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ResourcePut {
    const ID: &'static str = "resource.put";
    const TITLE: &'static str = "Add or replace a resource in a project";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ResourcePutParams;
    type Output = ResourceEntry;

    fn destination(
        &self,
        target: Option<&ResourceRef>,
        _params: &ResourcePutParams,
    ) -> Option<String> {
        Some(self.projects.canonical(target?.as_str()))
    }

    fn summary(&self, output: &ResourceEntry) -> Value {
        json!({ "name": output.name })
    }

    async fn run(
        &self,
        context: &Context,
        params: ResourcePutParams,
    ) -> Result<ResourceEntry, ActionError> {
        let target = context.target().ok_or(ActionError::TargetRequired)?.clone();
        let admitted = context.destination().map(str::to_owned);
        let projects = self.projects.clone();

        blocking(move || projects.put(target.as_str(), admitted.as_deref(), params.resource))
            .await
            .map_err(|error| ActionError::Failed(error.to_string()))
    }
}
