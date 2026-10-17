use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use synt_kube::{
    Cluster, ClusterObject, ClusterSpec, Kind, KindName, KubeError, KubeconfigSource, ListQuery,
    Manifest, ObjectName,
};
use synt_test_servers::KubeServers;
use synt_testkit::TestError;
use tokio::time;

const PATIENCE: Duration = Duration::from_secs(180);

pub struct Lab {
    pub cluster: Cluster,
    pub namespace: String,
}

impl Lab {
    pub async fn start() -> Result<Option<Self>, TestError> {
        let Some(servers) = KubeServers::running() else {
            return Ok(None);
        };
        let namespace = unique_namespace();

        let spec = spec(&servers, KubeServers::DIRECT).namespace(&namespace);
        let cluster = Cluster::connect(&spec, None).await?;

        let lab = Self { cluster, namespace };
        lab.apply(&format!(
            "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {}\n",
            lab.namespace
        ))
        .await?;

        Ok(Some(lab))
    }

    pub async fn apply(&self, yaml: &str) -> Result<(), KubeError> {
        let manifest = Manifest::parse(yaml)?;
        manifest.apply(&self.cluster, false).await?;

        Ok(())
    }

    pub async fn kind(&self, name: &str) -> Result<Kind, KubeError> {
        let name = KindName::try_from(name)?;

        self.cluster.kind(&name).await
    }

    pub fn object(&self, kind: &Kind, name: &str) -> Result<ClusterObject, KubeError> {
        let name = ObjectName::try_from(name)?;

        Ok(self.cluster.object(kind, &name, None))
    }

    pub async fn running_pod(&self, selector: &str) -> Result<ObjectName, TestError> {
        let pods = self.kind("pods").await?;
        let query = ListQuery {
            selector: Some(selector.to_owned()),
            ..ListQuery::default()
        };

        time::timeout(PATIENCE, async {
            loop {
                let listing = query.run(&self.cluster, &pods).await?;
                let running = listing.items.iter().find(|pod| {
                    pod["status"]["phase"] == "Running"
                        && pod["metadata"]["deletionTimestamp"].is_null()
                });

                if let Some(pod) = running {
                    let name = pod["metadata"]["name"]
                        .as_str()
                        .ok_or("the pod has no name")?;

                    return Ok(ObjectName::try_from(name)?);
                }

                time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .map_err(|_| format!("no pod matching {selector} ran within {PATIENCE:?}"))?
    }

    pub async fn finish(self) -> Result<(), KubeError> {
        let namespaces = self.kind("namespaces").await?;
        self.object(&namespaces, &self.namespace)?.delete().await?;

        Ok(())
    }
}

pub fn spec(servers: &KubeServers, context: &str) -> ClusterSpec {
    ClusterSpec::new(KubeconfigSource::File(servers.kubeconfig())).context(context)
}

pub fn ticker(name: &str) -> String {
    format!(
        "apiVersion: apps/v1
kind: Deployment
metadata:
  name: {name}
  labels:
    app: {name}
spec:
  replicas: 1
  selector:
    matchLabels:
      app: {name}
  template:
    metadata:
      labels:
        app: {name}
    spec:
      terminationGracePeriodSeconds: 1
      containers:
      - name: main
        image: busybox:1.37
        ports:
        - name: web
          containerPort: 8080
        command:
        - sh
        - -c
        - mkdir -p /www; echo hello from {name} > /www/index.html; httpd -p 8080 -h /www; i=0; while true; do echo tick $i; i=$((i+1)); sleep 1; done
---
apiVersion: v1
kind: Service
metadata:
  name: {name}
spec:
  selector:
    app: {name}
  ports:
  - port: 80
    targetPort: web
"
    )
}

fn unique_namespace() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.subsec_nanos())
        .unwrap_or_default();

    format!("synt-{}-{nanos}", process::id())
}
