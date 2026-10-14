use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::failure::failed;
use super::object_names::{check_kind, check_namespace, check_object_name};
use super::{KindFacts, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sGet {
    kube: Kube,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct GetParams {
    pub kind: String,
    pub name: String,

    #[serde(default)]
    pub namespace: Option<String>,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sGet {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sGet {
    const ID: &'static str = "k8s.get";
    const TITLE: &'static str = "Read one object from a cluster";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = GetParams;
    type Output = Value;

    fn check(&self, _target: Option<&ResourceRef>, params: &GetParams) -> Result<(), ActionError> {
        check_kind(&params.kind)?;
        check_object_name("name", &params.name)?;
        check_namespace(params.namespace.as_deref())
    }

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut GetParams,
    ) -> Result<Option<Value>, ActionError> {
        params.facts = self.kube.kind_facts(target, &params.kind)?;

        Ok(None)
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &GetParams) -> Risk {
        if params.facts.is_some_and(|facts| facts.secret) {
            return Risk::Privileged;
        }

        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &GetParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Value) -> Value {
        let metadata = &output["metadata"];

        json!({
            "kind": output["kind"],
            "name": metadata["name"],
            "namespace": metadata["namespace"],
        })
    }

    async fn run(&self, context: &Context, params: GetParams) -> Result<Value, ActionError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &params.kind).await?;

        cluster
            .get(&kind, &params.name, params.namespace.as_deref())
            .await
            .map_err(failed)
    }
}
