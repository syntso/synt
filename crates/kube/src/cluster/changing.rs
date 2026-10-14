use jiff::Timestamp;
use kube::api::{DeleteParams, Patch, PatchParams};
use serde_json::json;

use super::Cluster;
use crate::{Deleted, Kind, KubeError, ObjectRef, Restarted, Scaled};

const RESTARTED_AT: &str = "kubectl.kubernetes.io/restartedAt";
const RESTARTABLE: [&str; 3] = ["Deployment", "StatefulSet", "DaemonSet"];
const APPS: &str = "apps";
const LARGEST_REPLICAS: u32 = 10_000;

impl Cluster {
    pub async fn scale(
        &self,
        kind: &Kind,
        name: &str,
        namespace: Option<&str>,
        replicas: u32,
    ) -> Result<Scaled, KubeError> {
        if replicas > LARGEST_REPLICAS {
            return Err(KubeError::invalid(format!(
                "replicas must be at most {LARGEST_REPLICAS}, got {replicas}"
            )));
        }

        let object = self.object_ref(kind, name, namespace);
        let api = kind.api(self.client(), object.namespace.as_deref());

        let before = api.get_scale(name).await?;
        let patch = json!({ "spec": { "replicas": replicas } });
        let after = api
            .patch_scale(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;

        Ok(Scaled {
            object,
            from: before.spec.and_then(|spec| spec.replicas),
            to: after
                .spec
                .and_then(|spec| spec.replicas)
                .unwrap_or_default(),
        })
    }

    pub async fn restart(
        &self,
        kind: &Kind,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<Restarted, KubeError> {
        if kind.group() != APPS || !RESTARTABLE.contains(&kind.name()) {
            return Err(KubeError::invalid(format!(
                "{kind} cannot be restarted; only deployments, stateful sets and daemon sets can"
            )));
        }

        let object = self.object_ref(kind, name, namespace);
        let api = kind.api(self.client(), object.namespace.as_deref());

        let restarted_at = Timestamp::now().to_string();
        let patch = json!({
            "spec": { "template": { "metadata": { "annotations": { RESTARTED_AT: restarted_at } } } }
        });
        api.patch(name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;

        Ok(Restarted {
            object,
            restarted_at,
        })
    }

    pub async fn delete(
        &self,
        kind: &Kind,
        name: &str,
        namespace: Option<&str>,
    ) -> Result<Deleted, KubeError> {
        let object = self.object_ref(kind, name, namespace);
        let api = kind.api(self.client(), object.namespace.as_deref());

        let outcome = api.delete(name, &DeleteParams::background()).await?;

        Ok(Deleted {
            object,
            gone: outcome.is_right(),
        })
    }

    pub(crate) fn object_ref(&self, kind: &Kind, name: &str, namespace: Option<&str>) -> ObjectRef {
        ObjectRef {
            kind: kind.name().to_owned(),
            name: name.to_owned(),
            namespace: self.namespace_for(kind, namespace),
        }
    }
}
