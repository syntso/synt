use synt_kube::{EventQuery, KindName, KubeError, ListQuery, Namespace, NamespaceScope};

use crate::Lab;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn objects_are_listed_and_read_by_kind_name() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let namespaces = lab.kind("ns").await;
    let all = ListQuery::default()
        .run(&lab.cluster, &namespaces)
        .await
        .unwrap();
    let ours = lab.object(&namespaces, &lab.namespace).get().await.unwrap();

    assert_eq!(all.kind, "Namespace");
    assert!(
        all.items
            .iter()
            .any(|item| item["metadata"]["name"] == lab.namespace.as_str())
    );
    assert_eq!(ours["kind"], "Namespace");
    assert_eq!(ours["apiVersion"], "v1");
    assert!(ours["metadata"]["managedFields"].is_null());

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_selector_and_a_page_size_narrow_a_list() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    for name in ["first", "second", "third"] {
        let tier = if name == "third" { "back" } else { "front" };
        lab.apply(&format!(
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: {name}\n  namespace: {}\n  labels:\n    tier: {tier}\n",
            lab.namespace
        ))
        .await;
    }
    let maps = lab.kind("configmaps").await;
    let ours = || NamespaceScope::Only(Namespace::try_from(lab.namespace.as_str()).unwrap());

    let front = ListQuery {
        scope: ours(),
        selector: Some("tier=front".to_owned()),
        ..ListQuery::default()
    };
    let one_page = ListQuery {
        scope: ours(),
        selector: Some("tier".to_owned()),
        limit: Some(2),
        ..ListQuery::default()
    };
    let selected = front.run(&lab.cluster, &maps).await.unwrap();
    let first_page = one_page.run(&lab.cluster, &maps).await.unwrap();
    let next = ListQuery {
        continue_token: first_page.continue_token.clone(),
        ..one_page
    };
    let second_page = next.run(&lab.cluster, &maps).await.unwrap();

    assert_eq!(selected.items.len(), 2);
    assert_eq!(first_page.items.len(), 2);
    assert!(first_page.continue_token.is_some());
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(second_page.continue_token, None);

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_missing_object_is_not_found() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let pods = lab.kind("pods").await;
    let missing = lab.object(&pods, "nobody").get().await.unwrap_err();
    let unknown = lab
        .cluster
        .kind(&KindName::try_from("gadgets").unwrap())
        .await
        .unwrap_err();

    assert!(matches!(missing, KubeError::NotFound(_)), "{missing}");
    assert!(matches!(unknown, KubeError::UnknownKind(_)), "{unknown}");

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn events_tell_what_happened_to_an_object() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.apply(&crate::lab::ticker("eventful")).await;
    let pod = lab.running_pod("app=eventful").await;

    let query = EventQuery {
        name: Some(pod.clone()),
        ..EventQuery::default()
    };
    let events = query.run(&lab.cluster).await.unwrap();

    assert!(
        events
            .iter()
            .any(|event| event.reason.as_deref() == Some("Started"))
    );
    assert!(events.iter().all(|event| event.object.name == pod.as_str()));

    lab.finish().await;
}
