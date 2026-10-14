use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::Deleted;

use super::failure::failed;
use super::object_params::ObjectParams;
use super::{KindFacts, Kube};

#[derive(Debug, Clone)]
pub(crate) struct K8sDelete {
    kube: Kube,
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
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sDelete {
    const ID: &'static str = "k8s.delete";
    const TITLE: &'static str = "Delete an object from a cluster";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = DeleteParams;
    type Output = Deleted;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &DeleteParams,
    ) -> Result<(), ActionError> {
        params.object.check()
    }

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
        let object = params.object;
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &object.kind).await?;

        if !kind.is_namespaced() && context.risk() != Risk::Privileged {
            return Err(ActionError::Failed(format!(
                "{kind} is cluster-scoped, which was not known when the call was admitted; \
                 run it again so it is asked as privileged"
            )));
        }

        cluster
            .delete(&kind, &object.name, object.namespace.as_deref())
            .await
            .map_err(failed)
    }
}
