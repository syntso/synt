use serde_json::Value;

use super::Cluster;
use super::placed::Placed;
use crate::apply::Comparison;
use crate::objects::{Fields, without_noise};
use crate::{AppliedObject, Change, Kind, KubeError, Manifest, ObjectRef, Plan, PlannedObject};

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
            let placed = self.placed(object).await?;

            applied.push(placed.apply(object, force).await?);
        }

        Ok(applied)
    }

    async fn plan_one(
        &self,
        object: &Value,
        force: bool,
        new_namespaces: &[String],
    ) -> Result<(PlannedObject, Kind), KubeError> {
        let placed = self.placed(object).await?;
        let waits_for_namespace = placed.waits_for(new_namespaces);

        let mut comparison = if waits_for_namespace {
            Comparison::new(None, without_noise(object.clone()))
        } else {
            placed.dry_run(object, force).await?
        };
        let change = comparison.change();

        if placed.kind().is_secret() {
            comparison.hide_secret_values();
        }

        let diff = comparison.unified(&placed.object().to_string());
        let kind = placed.kind().clone();
        let planned = PlannedObject {
            object: placed.into_object(),
            change,
            server_checked: !waits_for_namespace,
            diff,
        };

        Ok((planned, kind))
    }

    async fn placed(&self, object: &Value) -> Result<Placed, KubeError> {
        let api_version = object.text("apiVersion").unwrap_or_default();
        let kind_name = object.text("kind").unwrap_or_default();
        let kind = self.kinds().await?.by_api_version(api_version, kind_name)?;

        let namespace = self.namespace_for(&kind, object.metadata_text("namespace"));
        let api = kind.api(self.client(), namespace.as_deref());

        let object = ObjectRef {
            namespace,
            ..ObjectRef::of(object)
        };

        Ok(Placed::new(kind, object, api))
    }
}
