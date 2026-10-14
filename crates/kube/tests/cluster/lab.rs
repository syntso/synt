use std::process;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use synt_kube::{Cluster, ClusterSpec, KubeconfigSource, ListQuery, Manifest};
use synt_test_servers::KubeServers;
use tokio::time;

const PATIENCE: Duration = Duration::from_secs(180);

pub struct Lab {
    pub cluster: Cluster,
    pub namespace: String,
}

impl Lab {
    pub async fn start() -> Option<Self> {
        let servers = KubeServers::running()?;
        let namespace = unique_namespace();

        let spec = spec(&servers, KubeServers::DIRECT).namespace(&namespace);
        let cluster = Cluster::connect(&spec, None).await.unwrap();

        let lab = Self { cluster, namespace };
        lab.apply(&format!(
            "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {}\n",
            lab.namespace
        ))
        .await;

        Some(lab)
    }

    pub async fn apply(&self, yaml: &str) {
        let manifest = Manifest::parse(yaml).unwrap();

        self.cluster.apply(&manifest, false).await.unwrap();
    }

    pub async fn running_pod(&self, selector: &str) -> String {
        let pods = self.cluster.kind("pods").await.unwrap();
        let query = ListQuery {
            selector: Some(selector.to_owned()),
            ..ListQuery::default()
        };

        time::timeout(PATIENCE, async {
            loop {
                let listing = self.cluster.list(&pods, &query).await.unwrap();
                let running = listing.items.iter().find(|pod| {
                    pod["status"]["phase"] == "Running"
                        && pod["metadata"]["deletionTimestamp"].is_null()
                });

                if let Some(pod) = running {
                    return pod["metadata"]["name"].as_str().unwrap().to_owned();
                }

                time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("no pod matching {selector} ran within {PATIENCE:?}"))
    }

    pub async fn finish(self) {
        let namespaces = self.cluster.kind("namespaces").await.unwrap();

        self.cluster
            .delete(&namespaces, &self.namespace, None)
            .await
            .unwrap();
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
