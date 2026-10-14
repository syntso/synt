use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::{K8sError, KindFacts, Kube, ObjectParams};

#[derive(Debug, Clone)]
pub(crate) struct K8sGet {
    kube: Kube,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct GetParams {
    #[serde(flatten)]
    pub object: ObjectParams,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sGet {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn get(&self, context: &Context, object: &ObjectParams) -> Result<Value, K8sError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &object.kind).await?;

        Ok(object.on(&cluster, &kind).get().await?)
    }
}

impl Action for K8sGet {
    const ID: &'static str = "k8s.get";
    const TITLE: &'static str = "Read one object from a cluster";
    const RISK: Risk = Risk::Read;
    const TAKES_TARGET: bool = true;

    type Params = GetParams;
    type Output = Value;

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut GetParams,
    ) -> Result<Option<Value>, ActionError> {
        params.facts = self.kube.kind_facts(target, &params.object.kind)?;

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
        Ok(self.get(context, &params.object).await?)
    }
}
