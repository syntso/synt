use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{Replicas, Scaled};

use super::{Kube, ObjectParams};

use crate::CoreError;

#[derive(Debug, Clone)]
pub(crate) struct K8sScale {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ScaleParams {
    #[serde(flatten)]
    pub object: ObjectParams,
    pub replicas: Replicas,
}

impl K8sScale {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    async fn scale(&self, context: &Context, params: &ScaleParams) -> Result<Scaled, CoreError> {
        let object = &params.object;
        let cluster = self.kube.cluster(context).await?;
        let kind = self.kube.kind(context, &cluster, &object.kind).await?;

        Ok(object.on(&cluster, &kind).scale(params.replicas).await?)
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
        target: Option<&ResourceRef>,
        _params: &ScaleParams,
    ) -> Result<(), ActionError> {
        self.kube.reach(target)?;

        Ok(())
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
        Ok(self.scale(context, &params).await?)
    }
}
