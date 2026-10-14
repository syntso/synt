use data_encoding::HEXLOWER;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;
use synt_kube::{AppliedObject, Change, Manifest};

use super::Kube;
use super::failure::failed;
use super::kube::wait_for;

const NO_DRY_RUN: &str = "not run: the cluster is reached through a root login, which is asked \
                          for first; the diff shows once that connection is up";

#[derive(Debug, Clone)]
pub(crate) struct K8sApply {
    kube: Kube,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
pub(crate) struct ApplyParams {
    pub manifest: String,

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
}

impl Action for K8sApply {
    const ID: &'static str = "k8s.apply";
    const TITLE: &'static str = "Apply a manifest to a cluster with server-side apply";
    const RISK: Risk = Risk::Write;
    const TAKES_TARGET: bool = true;

    type Params = ApplyParams;
    type Output = Applied;

    fn check(
        &self,
        _target: Option<&ResourceRef>,
        params: &ApplyParams,
    ) -> Result<(), ActionError> {
        Manifest::parse(&params.manifest)
            .map(|_| ())
            .map_err(failed)
    }

    fn inspect(
        &self,
        target: Option<&ResourceRef>,
        params: &mut ApplyParams,
    ) -> Result<Option<Value>, ActionError> {
        let manifest = Manifest::parse(&params.manifest).map_err(failed)?;

        let Some(cluster) = self.kube.inspected_cluster(target)? else {
            return Ok(Some(json!({
                "objects": manifest.refs(),
                "force": params.force,
                "diff": Value::Null,
                "dry_run": NO_DRY_RUN,
            })));
        };

        let plan = wait_for(cluster.plan(&manifest, params.force))?.map_err(failed)?;

        Ok(Some(json!({
            "objects": plan.objects,
            "force": params.force,
            "diff": plan.diff(),
        })))
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
        let count = |change: Change| {
            output
                .objects
                .iter()
                .filter(|object| object.change == change)
                .count()
        };

        json!({
            "objects": output.objects,
            "created": count(Change::Create),
            "updated": count(Change::Update),
            "unchanged": count(Change::Unchanged),
        })
    }

    async fn run(&self, context: &Context, params: ApplyParams) -> Result<Applied, ActionError> {
        let manifest = Manifest::parse(&params.manifest).map_err(failed)?;
        let cluster = self.kube.cluster(context).await?;

        let objects = cluster
            .apply(&manifest, params.force)
            .await
            .map_err(failed)?;

        Ok(Applied { objects })
    }
}
