use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use crate::blocking::blocking;
use crate::projects::{Projects, Resource, ResourceEntry};

const SECRET_SCHEME: &str = "secret://";
const HIDDEN: &str = "?";
const CREDENTIAL_WORDS: [&str; 6] = ["password", "passwd", "token", "secret", "credential", "key"];

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

    fn recorded_params(&self, params: &Value) -> Value {
        recorded_resource(params)
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

        blocking(move || projects.put(target.as_str(), admitted.as_deref(), params.resource)).await
    }
}

fn recorded_resource(params: &Value) -> Value {
    let Some(fields) = params.as_object() else {
        return params.clone();
    };

    let recorded = fields
        .iter()
        .map(|(name, value)| (name.clone(), recorded_field(name, value)))
        .collect();

    Value::Object(recorded)
}

fn recorded_field(name: &str, value: &Value) -> Value {
    let name = name.to_ascii_lowercase();
    let credential = CREDENTIAL_WORDS.iter().any(|word| name.contains(word));
    let reference = value
        .as_str()
        .is_some_and(|text| text.starts_with(SECRET_SCHEME));

    if credential && !reference {
        return Value::String(HIDDEN.to_owned());
    }

    value.clone()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::recorded_resource as recorded;

    #[test]
    fn a_plain_password_is_hidden_and_a_secret_reference_kept() {
        let plain = json!({ "kind": "database", "engine": "mysql", "password": "hunter2" });
        let kept =
            json!({ "kind": "database", "engine": "mysql", "password": "secret://orders-db" });

        assert_eq!(
            recorded(&plain),
            json!({ "kind": "database", "engine": "mysql", "password": "?" })
        );
        assert_eq!(recorded(&kept), kept);
    }

    #[test]
    fn any_field_named_like_a_credential_is_hidden() {
        let params = json!({ "kind": "host", "address": "10.0.0.1", "api_token": "t0k", "private_key": "-----BEGIN" });

        assert_eq!(
            recorded(&params),
            json!({ "kind": "host", "address": "10.0.0.1", "api_token": "?", "private_key": "?" })
        );
    }
}
