use synt_kube::{Change, KubeError, Manifest, Replicas};
use synt_testkit::TestError;

use crate::Lab;
use crate::lab::ticker;

fn settings(namespace: &str, mode: &str) -> Result<Manifest, KubeError> {
    Manifest::parse(&format!(
        "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: settings\n  namespace: {namespace}\ndata:\n  mode: {mode}\n"
    ))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_dry_run_shows_the_change_before_it_is_applied() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    let first = settings(&lab.namespace, "slow")?;
    let planned = first.plan(&lab.cluster, false).await.unwrap();
    let maps = lab.kind("cm").await?;
    let still_missing = lab.object(&maps, "settings")?.get().await;

    assert_eq!(planned.objects[0].change, Change::Create);
    assert!(
        planned.diff().contains("+  mode: slow"),
        "{}",
        planned.diff()
    );
    assert!(still_missing.is_err(), "a dry run changes nothing");

    let applied = first.apply(&lab.cluster, false).await.unwrap();
    let again = first.plan(&lab.cluster, false).await.unwrap();
    let changed = settings(&lab.namespace, "fast")?
        .plan(&lab.cluster, false)
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

    lab.finish().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_secret_diff_never_shows_its_values() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    let secret = |password: &str| {
        Manifest::parse(&format!(
            "apiVersion: v1\nkind: Secret\nmetadata:\n  name: login\n  namespace: {}\nstringData:\n  password: {password}\n",
            lab.namespace
        ))
        .unwrap()
    };
    secret("first-password")
        .apply(&lab.cluster, false)
        .await
        .unwrap();

    let planned = secret("second-password")
        .plan(&lab.cluster, false)
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

    lab.finish().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_deployment_scales_restarts_and_goes_away() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    lab.apply(&ticker("scaled")).await?;
    let deployments = lab.kind("deploy").await?;
    let deployment = || lab.object(&deployments, "scaled").unwrap();

    let scaled = deployment()
        .scale(Replicas::try_from(3).unwrap())
        .await
        .unwrap();
    let restarted = deployment().restart().await.unwrap();
    let live = deployment().get().await.unwrap();
    let pods = lab.kind("pods").await?;
    let not_restartable = lab.object(&pods, "anything")?.restart().await;
    let deleted = deployment().delete().await.unwrap();

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

    lab.finish().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn objects_in_a_namespace_the_manifest_creates_are_planned_from_the_manifest()
-> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    let fresh = format!("{}-fresh", lab.namespace);
    let manifest = Manifest::parse(&format!(
        "apiVersion: v1\nkind: Namespace\nmetadata:\n  name: {fresh}\n---\n\
         apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: settings\n  namespace: {fresh}\ndata:\n  mode: fast\n"
    ))
    .unwrap();

    let first = manifest.plan(&lab.cluster, false).await.unwrap();
    manifest.apply(&lab.cluster, false).await.unwrap();
    let second = manifest.plan(&lab.cluster, false).await.unwrap();

    let checked: Vec<(Change, bool)> = first
        .objects
        .iter()
        .map(|planned| (planned.change, planned.server_checked))
        .collect();
    assert_eq!(checked, [(Change::Create, true), (Change::Create, false)]);
    assert!(first.diff().contains("+  mode: fast"), "{}", first.diff());
    assert!(second.objects.iter().all(|planned| planned.server_checked));
    assert_eq!(second.count(Change::Unchanged), 2);

    let namespaces = lab.kind("namespaces").await?;
    lab.object(&namespaces, &fresh)?.delete().await.unwrap();
    lab.finish().await?;

    Ok(())
}
