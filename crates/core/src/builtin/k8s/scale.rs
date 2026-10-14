use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::Scaled;

use super::Kube;
use super::failure::failed;
use super::object_params::ObjectParams;

#[derive(Debug, Clone)]
pub(crate) struct K8sScale {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ScaleParams {
    #[serde(flatten)]
    pub object: ObjectParams,
    pub replicas: u32,
}

impl K8sScale {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }
}

impl Action for K8sScale {
    const ID: &'static str = "k8s.scale";
    const TITLE: &'static str = "Set the number of replicas of a deployment or stateful set";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ScaleParams;
    type Output = Scaled;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &ScaleParams,
    ) -> Result<(), ActionError> {
        params.object.check()
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &ScaleParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ScaleParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn summary(&self, output: &Scaled) -> Value {
        serde_json::to_value(output).unwrap_or_default()
    }

    async fn run(&self, context: &Context, params: ScaleParams) -> Result<Scaled, ActionError> {
        let object = params.object;
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &object.kind).await?;

        cluster
            .scale(
                &kind,
                &object.name,
                object.namespace.as_deref(),
                params.replicas,
            )
            .await
            .map_err(failed)
    }
}
