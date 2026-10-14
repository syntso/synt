use std::time::Duration;

use synt_kube::{NoticeKind, WatchNotice, WatchRequest};
use tokio::time;

use crate::Lab;

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn a_watch_reports_what_exists_then_every_change() {
    let Some(lab) = Lab::start().await else {
        return;
    };

    let manifest = |name: &str| {
        format!(
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: {name}\n  namespace: {}\n  labels:\n    watched: \"yes\"\n",
            lab.namespace
        )
    };
    lab.apply(&manifest("before")).await;

    let maps = lab.cluster.kind("configmaps").await.unwrap();
    let request = WatchRequest {
        selector: Some("watched=yes".to_owned()),
        ..WatchRequest::default()
    };
    let mut watch = lab.cluster.watch(&maps, &request);

    let existing = next(&mut watch).await;
    let synced = next(&mut watch).await;

    lab.apply(&manifest("after")).await;
    let added = next(&mut watch).await;

    let deleted_name = lab.cluster.delete(&maps, "before", None).await.unwrap();
    let deleted = next(&mut watch).await;

    assert_eq!(
        (existing.kind, existing.name.as_deref()),
        (NoticeKind::Existing, Some("before"))
    );
    assert_eq!(synced.kind, NoticeKind::Synced);
    assert_eq!(
        (added.kind, added.name.as_deref()),
        (NoticeKind::Added, Some("after"))
    );
    assert_eq!(added.object.as_ref().unwrap()["kind"], "ConfigMap");
    assert_eq!(deleted.kind, NoticeKind::Deleted);
    assert_eq!(deleted.name, Some(deleted_name.object.name));

    lab.finish().await;
}

async fn next(watch: &mut synt_kube::Watch) -> WatchNotice {
    time::timeout(Duration::from_secs(30), watch.next())
        .await
        .expect("a notice within 30 s")
        .unwrap()
}
