use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::Deleted;

use super::{K8sActions, KindFacts, ObjectParams};

use crate::CoreError;

#[derive(Debug, Clone)]
pub(crate) struct K8sDelete {
    kube: K8sActions,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub(crate) struct DeleteParams {
    #[serde(flatten)]
    pub object: ObjectParams,

    #[serde(skip)]
    #[schemars(skip)]
    facts: Option<KindFacts>,
}

impl K8sDelete {
    pub(super) fn new(kube: K8sActions) -> Self {
        Self { kube }
    }

    async fn delete(&self, context: &Context, object: &ObjectParams) -> Result<Deleted, CoreError> {
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &object.kind).await?;

        if !kind.is_namespaced() && context.risk() != Risk::Privileged {
            return Err(CoreError::ClusterScopedNow(kind.to_string()));
        }

        Ok(object.on(&cluster, &kind).delete().await?)
    }
}

impl Action for K8sDelete {
    const ID: &'static str = "k8s.delete";
    const TITLE: &'static str = "Delete an object from a cluster";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = DeleteParams;
    type Output = Deleted;

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut DeleteParams,
    ) -> Result<Option<Value>, ActionError> {
        params.facts = self.kube.kind_facts(target, &params.object.kind)?;

        Ok(None)
    }

    fn risk(&self, target: Option<&ResourceRef>, params: &DeleteParams) -> Risk {
        if params.facts.is_none_or(|facts| !facts.namespaced) {
            return Risk::Privileged;
        }

        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &DeleteParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Deleted) -> Value {
        serde_json::to_value(output).unwrap_or_default()
    }

    async fn run(&self, context: &Context, params: DeleteParams) -> Result<Deleted, ActionError> {
        Ok(self.delete(context, &params.object).await?)
    }
}
