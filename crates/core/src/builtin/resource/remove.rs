use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::blocking::blocking;
use crate::projects::{Projects, ResourceEntry, TargetRef};

#[derive(Debug, Clone)]
pub struct ResourceRemove {
    projects: Projects,
}

impl ResourceRemove {
    pub fn new(projects: Projects) -> Self {
        Self { projects }
    }
}

impl Action for ResourceRemove {
    const ID: &'static str = "resource.remove";
    const TITLE: &'static str = "Remove a resource from a project";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ();
    type Output = ResourceEntry;

    fn destination(&self, target: Option<&ResourceRef>, (): &()) -> Option<String> {
        Some(self.projects.canonical(target?.as_str()))
    }

    fn summary(&self, output: &ResourceEntry) -> Value {
        json!({ "name": output.name })
    }

    async fn run(&self, context: &Context, (): ()) -> Result<ResourceEntry, ActionError> {
        let target = context.target().ok_or(ActionError::TargetRequired)?.clone();
        let admitted = context.destination().map(str::to_owned);
        let projects = self.projects.clone();

        blocking(move || {
            let target = TargetRef::require(target.as_str())?;

            projects.remove(&target, admitted.as_deref())
        })
        .await
    }
}
