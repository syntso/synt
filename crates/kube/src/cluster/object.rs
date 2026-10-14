use jiff::Timestamp;
use kube::api::{Api, DeleteParams, DynamicObject, Patch, PatchParams};
use serde_json::{Value, json};

use crate::apply::Comparison;
use crate::objects::Noise;
use crate::{AppliedObject, Change, Deleted, Kind, KubeError, ObjectRef, Restarted, Scaled};

const FIELD_MANAGER: &str = "synt";
const RESTARTED_AT: &str = "kubectl.kubernetes.io/restartedAt";
const LARGEST_REPLICAS: u32 = 10_000;

pub struct ClusterObject {
    kind: Kind,
    reference: ObjectRef,
    api: Api<DynamicObject>,
}

impl ClusterObject {
    pub(super) fn new(kind: Kind, reference: ObjectRef, api: Api<DynamicObject>) -> Self {
        Self {
            kind,
            reference,
            api,
        }
    }

    pub fn kind(&self) -> &Kind {
        &self.kind
    }

    pub fn reference(&self) -> &ObjectRef {
        &self.reference
    }

    pub async fn get(&self) -> Result<Value, KubeError> {
        let object = self.api.get(self.name()).await?;

        Ok(self.kind.tidy(object))
    }

    pub async fn scale(self, replicas: u32) -> Result<Scaled, KubeError> {
        if replicas > LARGEST_REPLICAS {
            return Err(KubeError::invalid(format!(
                "replicas must be at most {LARGEST_REPLICAS}, got {replicas}"
            )));
        }

        let before = self.api.get_scale(self.name()).await?;
        let patch = json!({ "spec": { "replicas": replicas } });
        let after = self
            .api
            .patch_scale(self.name(), &PatchParams::default(), &Patch::Merge(&patch))
            .await?;

        Ok(Scaled {
            object: self.reference,
            from: before.spec.and_then(|spec| spec.replicas),
            to: after
                .spec
                .and_then(|spec| spec.replicas)
                .unwrap_or_default(),
        })
    }

    pub async fn restart(self) -> Result<Restarted, KubeError> {
        if !self.kind.is_restartable() {
            return Err(KubeError::invalid(format!(
                "{} cannot be restarted; only deployments, stateful sets and daemon sets can",
                self.kind
            )));
        }

        let restarted_at = Timestamp::now().to_string();
        let patch = json!({
            "spec": { "template": { "metadata": { "annotations": { RESTARTED_AT: restarted_at } } } }
        });
        self.api
            .patch(self.name(), &PatchParams::default(), &Patch::Merge(&patch))
            .await?;

        Ok(Restarted {
            object: self.reference,
            restarted_at,
        })
    }

    pub async fn delete(self) -> Result<Deleted, KubeError> {
        let outcome = self
            .api
            .delete(self.name(), &DeleteParams::background())
            .await?;

        Ok(Deleted {
            object: self.reference,
            gone: outcome.is_right(),
        })
    }

    pub(crate) fn into_reference(self) -> ObjectRef {
        self.reference
    }

    pub(crate) fn waits_for(&self, new_namespaces: &[String]) -> bool {
        self.reference
            .namespace
            .as_ref()
            .is_some_and(|namespace| new_namespaces.contains(namespace))
    }

    pub(crate) async fn dry_run(
        &self,
        object: &Value,
        force: bool,
    ) -> Result<Comparison, KubeError> {
        let live = self.live().await?;
        let dry_run = self.patch(object, &patch_params(force).dry_run()).await?;

        let before = live.map(|live| self.kind.tidy(live).without_noise());
        let after = self.kind.tidy(dry_run).without_noise();

        Ok(Comparison::new(before, after))
    }

    pub(crate) async fn apply(
        self,
        object: &Value,
        force: bool,
    ) -> Result<AppliedObject, KubeError> {
        let live = self.live().await?;
        let applied = self.patch(object, &patch_params(force)).await?;

        let change = match live.and_then(|live| live.metadata.resource_version) {
            None => Change::Create,
            Some(version) if applied.metadata.resource_version.as_ref() == Some(&version) => {
                Change::Unchanged
            }
            Some(_) => Change::Update,
        };

        Ok(AppliedObject {
            object: self.reference,
            change,
        })
    }

    fn name(&self) -> &str {
        &self.reference.name
    }

    async fn live(&self) -> Result<Option<DynamicObject>, KubeError> {
        Ok(self.api.get_opt(self.name()).await?)
    }

    async fn patch(
        &self,
        object: &Value,
        params: &PatchParams,
    ) -> Result<DynamicObject, KubeError> {
        self.api
            .patch(self.name(), params, &Patch::Apply(object))
            .await
            .map_err(|error| KubeError::from(error).concerning(&self.reference))
    }
}

fn patch_params(force: bool) -> PatchParams {
    let params = PatchParams::apply(FIELD_MANAGER);

    if force { params.force() } else { params }
}
