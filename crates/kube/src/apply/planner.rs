use serde_json::Value;

use super::Comparison;
use crate::objects::Noise;
use crate::{Change, Cluster, ClusterObject, KubeError, Manifest, Plan, PlannedObject};

pub(super) struct Planner<'cluster> {
    cluster: &'cluster Cluster,
    force: bool,
    new_namespaces: Vec<String>,
}

impl<'cluster> Planner<'cluster> {
    pub(super) fn new(cluster: &'cluster Cluster, force: bool) -> Self {
        Self {
            cluster,
            force,
            new_namespaces: Vec::new(),
        }
    }

    pub(super) async fn plan(mut self, manifest: &Manifest) -> Result<Plan, KubeError> {
        let mut objects = Vec::with_capacity(manifest.objects().len());

        for object in manifest.objects() {
            objects.push(self.plan_object(object).await?);
        }

        Ok(Plan { objects })
    }

    async fn plan_object(&mut self, object: &Value) -> Result<PlannedObject, KubeError> {
        let placed = self.cluster.object_in_manifest(object).await?;
        let server_checked = !placed.waits_for(&self.new_namespaces);

        let mut comparison = self.compare(&placed, object, server_checked).await?;
        let change = comparison.change();

        if placed.kind().is_secret() {
            comparison.hide_secret_values();
        }

        let diff = comparison.unified(&placed.reference().to_string());
        self.note_new_namespace(&placed, change);

        Ok(PlannedObject {
            object: placed.into_reference(),
            change,
            server_checked,
            diff,
        })
    }

    async fn compare(
        &self,
        placed: &ClusterObject,
        object: &Value,
        server_checked: bool,
    ) -> Result<Comparison, KubeError> {
        if !server_checked {
            return Ok(Comparison::new(None, object.clone().without_noise()));
        }

        placed.dry_run(object, self.force).await
    }

    fn note_new_namespace(&mut self, placed: &ClusterObject, change: Change) {
        if placed.kind().is_namespace() && change == Change::Create {
            self.new_namespaces.push(placed.reference().name.clone());
        }
    }
}
