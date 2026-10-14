use kube::api::{Api, DynamicObject, Patch, PatchParams};
use serde_json::Value;

use super::Cluster;
use crate::apply::{hide_secret_values, metadata_text, object_ref, text, unified};
use crate::objects::without_noise;
use crate::{AppliedObject, Change, Kind, KubeError, Manifest, ObjectRef, Plan, PlannedObject};

const FIELD_MANAGER: &str = "synt";

struct Placed {
    kind: Kind,
    object: ObjectRef,
    api: Api<DynamicObject>,
}

impl Cluster {
    pub async fn plan(&self, manifest: &Manifest, force: bool) -> Result<Plan, KubeError> {
        let mut objects = Vec::new();
        let mut new_namespaces = Vec::new();

        for object in manifest.objects() {
            let (planned, kind) = self.plan_one(object, force, &new_namespaces).await?;

            if kind.is_namespace() && planned.change == Change::Create {
                new_namespaces.push(planned.object.name.clone());
            }

            objects.push(planned);
        }

        Ok(Plan { objects })
    }

    pub async fn apply(
        &self,
        manifest: &Manifest,
        force: bool,
    ) -> Result<Vec<AppliedObject>, KubeError> {
        let mut applied = Vec::new();

        for object in manifest.objects() {
            applied.push(self.apply_one(object, force).await?);
        }

        Ok(applied)
    }

    async fn plan_one(
        &self,
        object: &Value,
        force: bool,
        new_namespaces: &[String],
    ) -> Result<(PlannedObject, Kind), KubeError> {
        let placed = self.place_object(object).await?;
        let waits_for_namespace = placed
            .object
            .namespace
            .as_ref()
            .is_some_and(|namespace| new_namespaces.contains(namespace));

        let (mut before, mut after) = if waits_for_namespace {
            (None, without_noise(object.clone()))
        } else {
            self.dry_run(&placed, object, force).await?
        };

        let change = match &before {
            None => Change::Create,
            Some(before) if *before == after => Change::Unchanged,
            Some(_) => Change::Update,
        };

        if placed.kind.is_secret() {
            hide_secret_values(before.as_mut(), &mut after);
        }

        let planned = PlannedObject {
            diff: unified(&placed.object.to_string(), before.as_ref(), &after),
            object: placed.object,
            change,
            server_checked: !waits_for_namespace,
        };

        Ok((planned, placed.kind))
    }

    async fn dry_run(
        &self,
        placed: &Placed,
        object: &Value,
        force: bool,
    ) -> Result<(Option<Value>, Value), KubeError> {
        let live = placed.api.get_opt(&placed.object.name).await?;
        let params = patch_params(force).dry_run();
        let dry_run = placed
            .api
            .patch(&placed.object.name, &params, &Patch::Apply(object))
            .await
            .map_err(|error| refused(&placed.object, error))?;

        let before = live.map(|live| without_noise(placed.kind.tidy(live)));
        let after = without_noise(placed.kind.tidy(dry_run));

        Ok((before, after))
    }

    async fn apply_one(&self, object: &Value, force: bool) -> Result<AppliedObject, KubeError> {
        let placed = self.place_object(object).await?;

        let live = placed.api.get_opt(&placed.object.name).await?;
        let applied = placed
            .api
            .patch(
                &placed.object.name,
                &patch_params(force),
                &Patch::Apply(object),
            )
            .await
            .map_err(|error| refused(&placed.object, error))?;

        let change = match live.and_then(|live| live.metadata.resource_version) {
            None => Change::Create,
            Some(version) if applied.metadata.resource_version.as_ref() == Some(&version) => {
                Change::Unchanged
            }
            Some(_) => Change::Update,
        };

        Ok(AppliedObject {
            object: placed.object,
            change,
        })
    }

    async fn place_object(&self, object: &Value) -> Result<Placed, KubeError> {
        let api_version = text(object, "apiVersion").unwrap_or_default();
        let kind_name = text(object, "kind").unwrap_or_default();
        let kind = self.kinds().await?.by_api_version(api_version, kind_name)?;

        let namespace = self.namespace_for(&kind, metadata_text(object, "namespace"));
        let api = kind.api(self.client(), namespace.as_deref());

        let object = ObjectRef {
            namespace,
            ..object_ref(object)
        };

        Ok(Placed { kind, object, api })
    }
}

fn patch_params(force: bool) -> PatchParams {
    let params = PatchParams::apply(FIELD_MANAGER);

    if force { params.force() } else { params }
}

fn refused(object: &ObjectRef, error: kube::Error) -> KubeError {
    match KubeError::from(error) {
        KubeError::Api(reason) => KubeError::Api(format!("{object}: {reason}")),
        KubeError::NotFound(reason) => KubeError::NotFound(format!("{object}: {reason}")),
        other => other,
    }
}
