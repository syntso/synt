use serde_json::json;
use synt_proto::Risk;
use synt_testkit::TestError;

use crate::Lab;
use crate::lab::{DIRECT, INSIDE, ticker};

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn an_apply_asks_with_the_dry_run_diff_and_journals_no_manifest() -> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    let manifest = |mode: &str| {
        format!(
            "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: settings\n  namespace: {}\ndata:\n  mode: {mode}\n",
            lab.namespace
        )
    };
    lab.person(DIRECT, "k8s.apply", json!({ "manifest": manifest("slow") }))
        .await
        .unwrap();

    let (question, applied) = lab
        .agent_asking(INSIDE, "k8s.apply", json!({ "manifest": manifest("fast") }))
        .await?;
    let applied = applied.unwrap();
    let requested = lab.requested(applied.request);
    let finished = lab.finished(applied.request).await;

    let diff = question.params["diff"].as_str().unwrap();
    assert_eq!(question.risk, Risk::Write);
    assert!(
        question
            .destination
            .as_deref()
            .unwrap()
            .contains("via synt@127.0.0.1")
    );
    assert!(
        diff.contains("-  mode: slow") && diff.contains("+  mode: fast"),
        "{diff}"
    );
    assert_eq!(question.params["objects"][0]["change"], "update");

    assert!(requested["params"].get("manifest").is_none());
    assert_eq!(requested["params"]["objects"][0]["name"], "settings");
    assert_eq!(
        requested["params"]["manifest_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );

    assert_eq!(applied.value["objects"][0]["change"], "update");
    assert_eq!(finished["updated"], 1);
    assert!(!lab.journal_text()?.contains("mode: fast"));

    lab.finish().await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs the kube test cluster: tests/kube/up.sh, then SYNT_KUBE_TESTS=1"]
async fn scaling_restarting_and_deleting_ask_an_agent_and_record_names_only()
-> Result<(), TestError> {
    let Some(lab) = Lab::start().await? else {
        return Ok(());
    };

    lab.person(DIRECT, "k8s.apply", json!({ "manifest": ticker("api") }))
        .await
        .unwrap();
    let object = json!({ "kind": "deploy", "name": "api" });

    let (scale_question, scaled) = lab
        .agent_asking(
            DIRECT,
            "k8s.scale",
            json!({ "kind": "deploy", "name": "api", "replicas": 2 }),
        )
        .await?;
    let (_, restarted) = lab
        .agent_asking(DIRECT, "k8s.rollout_restart", object.clone())
        .await?;
    let (delete_question, deleted) = lab.agent_asking(DIRECT, "k8s.delete", object).await?;
    let (namespace_question, refused) = lab
        .desk
        .decide(
            &lab.agent,
            crate::lab::call(
                DIRECT,
                "k8s.delete",
                json!({ "kind": "namespace", "name": lab.namespace }),
            ),
            synt_proto::ApprovalDecision::Deny,
        )
        .await?;

    let scaled = scaled.unwrap();
    assert_eq!(scale_question.risk, Risk::Write);
    assert_eq!(scaled.value["to"], 2);
    assert_eq!(
        lab.finished(scaled.request).await,
        json!({ "kind": "Deployment", "name": "api", "namespace": lab.namespace, "from": 1, "to": 2 })
    );
    assert!(lab.finished(restarted.unwrap().request).await["restarted_at"].is_string());
    assert_eq!(delete_question.risk, Risk::Write);
    assert_eq!(lab.finished(deleted.unwrap().request).await["name"], "api");
    assert_eq!(namespace_question.risk, Risk::Privileged);
    assert!(refused.is_err());

    lab.finish().await?;

    Ok(())
}
