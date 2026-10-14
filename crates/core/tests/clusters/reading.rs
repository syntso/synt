use serde_json::json;
use synt_proto::RiskLevel;
use synt_test_servers::KubeServers;

use crate::Lab;
use crate::lab::{DIRECT, INSIDE};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn the_personal_project_lists_every_kubeconfig_context_as_a_cluster() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let listed = lab
        .person("", "resource.list", json!({ "project": "personal" }))
        .await
        .unwrap();
    let contexts = lab.agent("", "k8s.contexts", json!({})).await.unwrap();

    let clusters: Vec<&str> = listed.value["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["kind"] == "cluster")
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    let names: Vec<&str> = contexts
        .value
        .as_array()
        .unwrap()
        .iter()
        .map(|context| context["name"].as_str().unwrap())
        .collect();

    assert_eq!(
        clusters,
        [
            KubeServers::DIRECT,
            KubeServers::INSIDE,
            KubeServers::PLUGIN
        ]
    );
    assert_eq!(names, clusters);
    assert!(!contexts.value.to_string().contains("token"));

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn an_agent_reads_without_asking_and_the_journal_keeps_only_counts() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let by_context = lab
        .agent(
            KubeServers::PLUGIN,
            "k8s.list",
            json!({ "kind": "namespaces" }),
        )
        .await
        .unwrap();
    let personal = lab
        .agent(
            "@personal/direct",
            "k8s.get",
            json!({ "kind": "ns", "name": lab.namespace }),
        )
        .await
        .unwrap();
    let events = lab.agent(DIRECT, "k8s.events", json!({})).await.unwrap();

    let summary = lab.finished(by_context.request).await;
    let requested = lab.requested(personal.request);

    assert!(by_context.value["items"].as_array().unwrap().len() >= 4);
    assert_eq!(
        summary,
        json!({ "kind": "Namespace", "items": by_context.value["items"].as_array().unwrap().len(), "more": false })
    );
    assert_eq!(personal.value["metadata"]["name"], lab.namespace.as_str());
    assert_eq!(
        lab.finished(personal.request).await,
        json!({ "kind": "Namespace", "name": lab.namespace, "namespace": null })
    );
    assert_eq!(requested["params"]["name"], lab.namespace.as_str());
    assert!(events.value.is_array());

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_cluster_behind_a_bastion_is_reached_through_an_ssh_tunnel() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let listed = lab
        .agent(INSIDE, "k8s.list", json!({ "kind": "namespaces" }))
        .await
        .unwrap();
    let journaled = lab.finished(listed.request).await;
    let unreachable = lab
        .agent("@lab/bastion", "k8s.list", json!({ "kind": "pods" }))
        .await
        .unwrap_err();

    assert!(listed.value["items"].as_array().unwrap().len() >= 4);
    assert_eq!(journaled["kind"], "Namespace");
    assert!(
        unreachable.to_string().contains("not a cluster"),
        "{unreachable}"
    );

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn secrets_are_privileged_and_their_values_never_reach_the_journal() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let manifest = format!(
        "apiVersion: v1\nkind: Secret\nmetadata:\n  name: login\n  namespace: {}\nstringData:\n  password: correct-horse-battery\n",
        lab.namespace
    );
    lab.person(DIRECT, "k8s.apply", json!({ "manifest": manifest }))
        .await
        .unwrap();

    let (asked, read) = lab
        .agent_asking(
            DIRECT,
            "k8s.get",
            json!({ "kind": "secret", "name": "login" }),
        )
        .await;
    let read = read.unwrap();
    let (listed_question, listed) = lab
        .agent_asking(DIRECT, "k8s.list", json!({ "kind": "secrets" }))
        .await;
    listed.unwrap();
    let configmaps = lab
        .agent(DIRECT, "k8s.list", json!({ "kind": "configmaps" }))
        .await;

    let text = lab.journal_text();

    assert_eq!(asked.risk, RiskLevel::Privileged);
    assert_eq!(listed_question.risk, RiskLevel::Privileged);
    assert_eq!(
        read.value["data"]["password"],
        "Y29ycmVjdC1ob3JzZS1iYXR0ZXJ5"
    );
    assert!(configmaps.is_ok(), "other kinds stay read: {configmaps:?}");
    assert!(!text.contains("correct-horse-battery"));
    assert!(!text.contains("Y29ycmVjdC1ob3JzZS1iYXR0ZXJ5"));

    lab.finish().await;
}
