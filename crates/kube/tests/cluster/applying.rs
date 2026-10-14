use synt_kube::{Change, Manifest};

use crate::Lab;
use crate::lab::ticker;

fn settings(namespace: &str, mode: &str) -> Manifest {
    Manifest::parse(&format!(
        "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: settings\n  namespace: {namespace}\ndata:\n  mode: {mode}\n"
    ))
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_dry_run_shows_the_change_before_it_is_applied() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let first = settings(&lab.namespace, "slow");
    let planned = lab.cluster.plan(&first, false).await.unwrap();
    let maps = lab.cluster.kind("cm").await.unwrap();
    let still_missing = lab.cluster.get(&maps, "settings", None).await;

    assert_eq!(planned.objects[0].change, Change::Create);
    assert!(
        planned.diff().contains("+  mode: slow"),
        "{}",
        planned.diff()
    );
    assert!(still_missing.is_err(), "a dry run changes nothing");

    let applied = lab.cluster.apply(&first, false).await.unwrap();
    let again = lab.cluster.plan(&first, false).await.unwrap();
    let changed = lab
        .cluster
        .plan(&settings(&lab.namespace, "fast"), false)
        .await
        .unwrap();

    assert_eq!(applied[0].change, Change::Create);
    assert_eq!(again.objects[0].change, Change::Unchanged);
    assert_eq!(again.diff(), "");
    assert_eq!(changed.objects[0].change, Change::Update);
    assert!(
        changed.diff().contains("-  mode: slow"),
        "{}",
        changed.diff()
    );
    assert!(
        changed.diff().contains("+  mode: fast"),
        "{}",
        changed.diff()
    );

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_secret_diff_never_shows_its_values() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let secret = |password: &str| {
        Manifest::parse(&format!(
            "apiVersion: v1\nkind: Secret\nmetadata:\n  name: login\n  namespace: {}\nstringData:\n  password: {password}\n",
            lab.namespace
        ))
        .unwrap()
    };
    lab.cluster
        .apply(&secret("first-password"), false)
        .await
        .unwrap();

    let planned = lab
        .cluster
        .plan(&secret("second-password"), false)
        .await
        .unwrap();
    let diff = planned.diff();

    assert_eq!(planned.objects[0].change, Change::Update);
    assert!(diff.contains("*** (after)"), "{diff}");
    assert!(
        !diff.contains("password: second") && !diff.contains("c2Vjb25k"),
        "{diff}"
    );
    assert!(!diff.contains("Zmlyc3Q"), "{diff}");

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_deployment_scales_restarts_and_goes_away() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    lab.apply(&ticker("scaled")).await;
    let deployments = lab.cluster.kind("deploy").await.unwrap();

    let scaled = lab
        .cluster
        .scale(&deployments, "scaled", None, 3)
        .await
        .unwrap();
    let restarted = lab
        .cluster
        .restart(&deployments, "scaled", None)
        .await
        .unwrap();
    let live = lab.cluster.get(&deployments, "scaled", None).await.unwrap();
    let pods = lab.cluster.kind("pods").await.unwrap();
    let not_restartable = lab.cluster.restart(&pods, "anything", None).await;
    let deleted = lab
        .cluster
        .delete(&deployments, "scaled", None)
        .await
        .unwrap();

    assert_eq!((scaled.from, scaled.to), (Some(1), 3));
    assert_eq!(live["spec"]["replicas"], 3);
    assert_eq!(
        live["spec"]["template"]["metadata"]["annotations"]["kubectl.kubernetes.io/restartedAt"],
        restarted.restarted_at.as_str()
    );
    assert!(not_restartable.is_err());
    assert_eq!(deleted.object.name, "scaled");
    assert_eq!(
        deleted.object.namespace.as_deref(),
        Some(lab.namespace.as_str())
    );

    lab.finish().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn objects_in_a_namespace_the_manifest_creates_are_planned_from_the_manifest() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let fresh = format!("{}-fresh", lab.namespace);
    let manifest = Manifest::parse(&format!(
        "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {fresh}\n---\n\
         apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: settings\n  namespace: {fresh}\ndata:\n  mode: fast\n"
    ))
    .unwrap();

    let first = lab.cluster.plan(&manifest, false).await.unwrap();
    lab.cluster.apply(&manifest, false).await.unwrap();
    let second = lab.cluster.plan(&manifest, false).await.unwrap();

    let checked: Vec<(Change, bool)> = first
        .objects
        .iter()
        .map(|planned| (planned.change, planned.server_checked))
        .collect();
    assert_eq!(checked, [(Change::Create, true), (Change::Create, false)]);
    assert!(first.diff().contains("+  mode: fast"), "{}", first.diff());
    assert!(second.objects.iter().all(|planned| planned.server_checked));
    assert_eq!(second.count(Change::Unchanged), 2);

    let namespaces = lab.cluster.kind("namespaces").await.unwrap();
    lab.cluster.delete(&namespaces, &fresh, None).await.unwrap();
    lab.finish().await;
}
