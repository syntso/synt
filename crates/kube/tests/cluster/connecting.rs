use std::fs;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use synt_kube::{Cluster, ClusterSpec, KubeError, KubeconfigSource, ListQuery};
use synt_test_servers::KubeServers;

use crate::TcpDialer;
use crate::lab::spec;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_kubeconfig_context_reaches_the_cluster() {
    let Some(servers) = KubeServers::running() else {
        return;
    };

    let cluster = Cluster::connect(&spec(&servers, KubeServers::DIRECT), None)
        .await
        .unwrap();

    assert!(cluster.version().await.unwrap().starts_with("v1."));
    assert_eq!(cluster.place().context, KubeServers::DIRECT);
    assert_eq!(cluster.namespace(), "default");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn an_exec_credential_plugin_logs_in() {
    let Some(servers) = KubeServers::running() else {
        return;
    };

    let cluster = Cluster::connect(&spec(&servers, KubeServers::PLUGIN), None)
        .await
        .unwrap();
    let namespaces = cluster.kind("namespaces").await.unwrap();
    let listing = cluster
        .list(&namespaces, &ListQuery::default())
        .await
        .unwrap();

    assert!(
        listing
            .items
            .iter()
            .any(|item| item["metadata"]["name"] == "kube-system")
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_dialed_route_checks_tls_against_the_name_in_the_kubeconfig() {
    let Some(servers) = KubeServers::running() else {
        return;
    };

    let dialer = TcpDialer::to_local_port(servers.api_port());
    let dials = dialer.counter();
    let cluster = Cluster::connect(&spec(&servers, KubeServers::INSIDE), Some(Arc::new(dialer)))
        .await
        .unwrap();

    assert!(cluster.version().await.unwrap().starts_with("v1."));
    assert!(dials.load(Ordering::Relaxed) >= 1);
    assert_eq!(cluster.place().server, "https://cluster:6443");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_dialed_route_to_a_name_the_certificate_does_not_hold_is_refused() {
    let Some(servers) = KubeServers::running() else {
        return;
    };

    let folder = tempfile::tempdir().unwrap();
    let kubeconfig = folder.path().join("kubeconfig");
    let renamed = servers
        .kubeconfig_text()
        .replace("https://cluster:6443", "https://impostor:6443");
    fs::write(&kubeconfig, renamed).unwrap();

    let spec = ClusterSpec::new(KubeconfigSource::File(kubeconfig)).context(KubeServers::INSIDE);
    let dialer = TcpDialer::to_local_port(servers.api_port());
    let cluster = Cluster::connect(&spec, Some(Arc::new(dialer)))
        .await
        .unwrap();

    let refused = cluster
        .version()
        .await
        .expect_err("the certificate does not name impostor");

    assert!(matches!(refused, KubeError::Connect(_)), "{refused}");
}

#[test]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
fn contexts_are_listed_without_credentials() {
    let Some(servers) = KubeServers::running() else {
        return;
    };

    let contexts = KubeconfigSource::File(servers.kubeconfig())
        .contexts()
        .unwrap();
    let names: Vec<&str> = contexts
        .iter()
        .map(|context| context.name.as_str())
        .collect();
    let shown = serde_json::to_string(&contexts).unwrap();

    assert_eq!(
        names,
        [
            KubeServers::DIRECT,
            KubeServers::INSIDE,
            KubeServers::PLUGIN
        ]
    );
    assert!(contexts[0].current && !contexts[1].current);
    assert_eq!(contexts[1].server.as_deref(), Some("https://cluster:6443"));
    assert!(!shown.contains("data") && !shown.contains("token") && !shown.contains("exec"));
}

#[test]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
fn an_unknown_context_is_named_in_the_error() {
    let Some(servers) = KubeServers::running() else {
        return;
    };

    let error = spec(&servers, "nowhere").place().unwrap_err();

    assert!(matches!(error, KubeError::UnknownContext { ref context, .. } if context == "nowhere"));
}
