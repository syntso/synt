use kube::api::{Api, DynamicObject, Patch, PatchParams};
use serde_json::Value;

use crate::apply::Comparison;
use crate::objects::without_noise;
use crate::{AppliedObject, Change, Kind, KubeError, ObjectRef};

const FIELD_MANAGER: &str = "synt";

pub(super) struct Placed {
    kind: Kind,
    object: ObjectRef,
    api: Api<DynamicObject>,
}

impl Placed {
    pub(super) fn new(kind: Kind, object: ObjectRef, api: Api<DynamicObject>) -> Self {
        Self { kind, object, api }
    }

    pub(super) fn kind(&self) -> &Kind {
        &self.kind
    }

    pub(super) fn object(&self) -> &ObjectRef {
        &self.object
    }

    pub(super) fn into_object(self) -> ObjectRef {
        self.object
    }

    pub(super) fn waits_for(&self, new_namespaces: &[String]) -> bool {
        self.object
            .namespace
            .as_ref()
            .is_some_and(|namespace| new_namespaces.contains(namespace))
    }

    pub(super) async fn dry_run(
        &self,
        object: &Value,
        force: bool,
    ) -> Result<Comparison, KubeError> {
        let live = self.live().await?;
        let dry_run = self.patch(object, &patch_params(force).dry_run()).await?;

        let before = live.map(|live| without_noise(self.kind.tidy(live)));
        let after = without_noise(self.kind.tidy(dry_run));

        Ok(Comparison::new(before, after))
    }

    pub(super) async fn apply(
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
            object: self.object,
            change,
        })
    }

    async fn live(&self) -> Result<Option<DynamicObject>, KubeError> {
        Ok(self.api.get_opt(&self.object.name).await?)
    }

    async fn patch(
        &self,
        object: &Value,
        params: &PatchParams,
    ) -> Result<DynamicObject, KubeError> {
        self.api
            .patch(&self.object.name, params, &Patch::Apply(object))
            .await
            .map_err(|error| KubeError::from(error).concerning(&self.object))
    }
}

fn patch_params(force: bool) -> PatchParams {
    let params = PatchParams::apply(FIELD_MANAGER);

    if force { params.force() } else { params }
}
