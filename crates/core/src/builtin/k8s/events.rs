use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{EventLine, Namespaces};

use super::Kube;
use super::failure::failed;
use super::object_names::{check_namespace, check_object_name};

#[derive(Debug, Clone)]
pub(crate) struct K8sEvents {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct EventsParams {
    #[serde(default)]
    pub namespace: Option<String>,

    #[serde(default)]
    pub all_namespaces: bool,

    #[serde(default)]
    pub name: Option<String>,
}

impl K8sEvents {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sEvents {
    const ID: &'static str = "k8s.events";
    const TITLE: &'static str = "Read the events of a namespace or of one object";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = EventsParams;
    type Output = Vec<EventLine>;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &EventsParams,
    ) -> Result<(), ActionError> {
        check_namespace(params.namespace.as_deref())?;

        match &params.name {
            Some(name) => check_object_name("name", name),
            None => Ok(()),
        }
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &EventsParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &EventsParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Vec<EventLine>) -> Value {
        json!({ "events": output.len() })
    }

    async fn run(
        &self,
        context: &Context,
        params: EventsParams,
    ) -> Result<Vec<EventLine>, ActionError> {
        let cluster = self.kube.cluster(context).await?;
        let namespaces = Namespaces::from_request(params.namespace, params.all_namespaces);

        cluster
            .events(&namespaces, params.name.as_deref())
            .await
            .map_err(failed)
    }
}
