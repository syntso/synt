use data_encoding::HEXLOWER;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{AppliedObject, Change, Manifest};

use super::{K8sError, Kube};

const NO_DRY_RUN: &str = "not run: the cluster is reached through a root login, which is asked \
                          for first; the diff shows once that connection is up";

#[derive(Debug, Clone)]
pub(crate) struct K8sApply {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub(crate) struct ApplyParams {
    pub manifest: Manifest,

    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Applied {
    pub objects: Vec<AppliedObject>,
}

impl K8sApply {
    pub(super) fn new(kube: Kube) -> Self {
        Self { kube }
    }

    fn preview(
        &self,
        target: Option<&ResourceRef>,
        params: &ApplyParams,
    ) -> Result<Value, K8sError> {
        let manifest = &params.manifest;
        let plan = self.kube.inspect(target, |cluster| async move {
            manifest.plan(&cluster, params.force).await
        })?;

        let Some(plan) = plan else {
            return Ok(json!({
                "objects": manifest.refs(),
                "force": params.force,
                "diff": Value::Null,
                "dry_run": NO_DRY_RUN,
            }));
        };

        Ok(json!({
            "objects": plan.objects,
            "force": params.force,
            "diff": plan.diff(),
        }))
    }

    async fn apply(&self, context: &Context, params: &ApplyParams) -> Result<Applied, K8sError> {
        let cluster = self.kube.cluster(context).await?;
        let objects = params.manifest.apply(&cluster, params.force).await?;

        Ok(Applied { objects })
    }
}

impl Applied {
    fn count(&self, change: Change) -> usize {
        self.objects
            .iter()
            .filter(|object| object.change == change)
            .count()
    }
}

impl Action for K8sApply {
    const ID: &'static str = "k8s.apply";
    const TITLE: &'static str = "Apply a manifest to a cluster with server-side apply";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ApplyParams;
    type Output = Applied;

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut ApplyParams,
    ) -> Result<Option<Value>, ActionError> {
        Ok(Some(self.preview(target, params)?))
    }

    fn risk(&self, target: Option<&ResourceRef>, _params: &ApplyParams) -> Risk {
        self.kube.risk(target, Self::RISK)
    }

    fn destination(&self, target: Option<&ResourceRef>, _params: &ApplyParams) -> Option<String> {
        self.kube.destination(target)
    }

    fn recorded_params(&self, params: &Value) -> Value {
        let text = params["manifest"].as_str().unwrap_or_default();
        let objects = Manifest::parse(text).map_or(Value::Null, |manifest| json!(manifest.refs()));

        json!({
            "objects": objects,
            "force": params["force"],
            "manifest_bytes": text.len(),
            "manifest_sha256": HEXLOWER.encode(&Sha256::digest(text.as_bytes())),
        })
    }

    fn summary(&self, output: &Applied) -> Value {
        json!({
            "objects": output.objects,
            "created": output.count(Change::Create),
            "updated": output.count(Change::Update),
            "unchanged": output.count(Change::Unchanged),
        })
    }

    async fn run(&self, context: &Context, params: ApplyParams) -> Result<Applied, ActionError> {
        Ok(self.apply(context, &params).await?)
    }
}
