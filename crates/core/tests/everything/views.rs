use data_encoding::BASE64;
use serde_json::json;
use synt_journal::EventKind;
use synt_proto::{ActionCall, ErrorCode, Notice, Notification, PanelPosted, ViewPatched};
use synt_testkit::TestError;
use tokio::sync::broadcast;

use super::Stage;
use super::stage::read;
use synt_testkit::core::rejection;

async fn next<T: Notice>(
    notifications: &mut broadcast::Receiver<Notification>,
) -> Result<T, TestError> {
    loop {
        let notification = notifications.recv().await?;

        if let Some(found) = read::<T>(&notification) {
            return Ok(found);
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_app_learns_what_the_extension_contributes() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let contributed = stage.call("extension.contributions", json!(null)).await?;
    let every = &contributed[0];

    assert_eq!(every["extension"], "every");
    assert_eq!(every["contributes"]["sidebar"][0]["view"], "overview");
    assert_eq!(every["contributes"]["panel"][0]["entry"], "index.html");
    assert_eq!(every["contributes"]["document"][0]["suffix"], "note");
    assert_eq!(every["contributes"]["command"][0]["action"], "every.ping");
    assert_eq!(
        every["contributes"]["resource"]["menu"][0]["kinds"],
        json!(["host"])
    );
    assert_eq!(every["contributes"]["status"][0]["id"], "beat");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_native_view_renders_for_a_target_and_is_journaled_without_its_tree()
-> Result<(), TestError> {
    let stage = Stage::start().await?;

    let rendered = stage
        .app()
        .call(
            ActionCall::new("extension.view.render")
                .target("web-1")
                .params(json!({ "extension": "every", "view": "overview" })),
        )
        .await
        .unwrap();
    let finished = stage
        .trail(synt_journal::Seq::new(rendered.request.get()))?
        .into_iter()
        .find(|event| event.kind() == EventKind::Finished)
        .unwrap();

    let tree = &rendered.value["tree"];
    assert_eq!(tree["children"][0]["text"], "Everything on web-1");
    assert_eq!(tree["children"][2]["action"], "every.ping");
    assert!(finished.details().value().get("tree").is_none());
    assert!(finished.details().value()["bytes"].as_u64().unwrap() > 0);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn views_the_manifest_does_not_contribute_are_refused() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let refused = stage
        .try_call(
            "extension.view.render",
            json!({ "extension": "every", "view": "hidden" }),
        )
        .await;

    assert_eq!(rejection(refused).code, ErrorCode::INVALID_PARAMS);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_extension_patches_its_view_on_every_app() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let (_watcher, mut notifications) = stage.watcher().await?;

    stage.every("patch", json!({})).await?;
    let patched: ViewPatched = next(&mut notifications).await?;

    assert_eq!(patched.view, "overview");
    assert_eq!(
        patched.patch,
        json!([{ "op": "replace", "path": "/children/0/text", "value": "patched" }])
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_web_panel_is_served_from_the_extension_under_a_strict_policy() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let panel = json!({ "extension": "every", "panel": "editor" });

    let opened = stage.call("extension.panel.open", panel).await?;
    let page = stage
        .call(
            "extension.panel.asset",
            json!({ "extension": "every", "path": "index.html" }),
        )
        .await?;
    let html = BASE64
        .decode(page["data"].as_str().unwrap().as_bytes())
        .unwrap();

    assert_eq!(opened["entry"], "index.html");
    assert!(
        opened["csp"]
            .as_str()
            .unwrap()
            .contains("connect-src 'none'")
    );
    assert_eq!(page["mime"], "text/html; charset=utf-8");
    assert!(String::from_utf8(html).unwrap().contains("<textarea"));

    for (path, code) in [
        ("../extension.toml", ErrorCode::INVALID_PARAMS),
        ("run.sh", ErrorCode::INVALID_PARAMS),
        ("missing.css", ErrorCode::ACTION_FAILED),
    ] {
        let refused = stage
            .try_call(
                "extension.panel.asset",
                json!({ "extension": "every", "path": path }),
            )
            .await;

        assert_eq!(rejection(refused).code, code, "{path}");
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panel_message_reaches_the_extension_and_its_calls_reach_the_core()
-> Result<(), TestError> {
    let stage = Stage::start().await?;

    let reply = stage
        .try_call(
            "extension.panel.message",
            json!({ "extension": "every", "panel": "editor", "message": { "hello": "editor" } }),
        )
        .await
        .unwrap();
    let nested = stage.nested("core.ping")?;

    assert_eq!(reply.value["echo"], json!({ "hello": "editor" }));
    assert!(reply.value["pong"]["core"].is_string());
    assert_eq!(
        nested[0].details().value()["parent"],
        json!(reply.request.get())
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_extension_posts_to_its_panel() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let (_watcher, mut notifications) = stage.watcher().await?;

    stage
        .every("post", json!({ "message": { "saved": "plan.note" } }))
        .await?;
    let posted: PanelPosted = next(&mut notifications).await?;

    assert_eq!(posted.extension, "every");
    assert_eq!(posted.panel, "editor");
    assert_eq!(posted.message, json!({ "saved": "plan.note" }));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn the_new_actions_are_described_with_risk_and_params() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let agent = stage.harness.agent("helper").await?;

    let for_person = stage.app().describe().await.unwrap();
    let for_agent = agent.describe().await.unwrap();
    let find = |id: &str| for_person.iter().find(|action| action.id == id).cloned();

    for id in [
        "extension.contributions",
        "extension.documents",
        "extension.file.forget",
        "extension.file.list",
        "extension.file.pick",
        "extension.http",
        "extension.panel.asset",
        "extension.panel.message",
        "extension.panel.open",
        "extension.settings.get",
        "extension.settings.set",
        "extension.view.render",
        "secret.list",
        "secret.put",
        "secret.remove",
    ] {
        let action = find(id).unwrap_or_else(|| panic!("{id} is not described"));
        assert!(!action.title.is_empty(), "{id}");
        assert_eq!(action.params["type"], "object", "{id}");
    }

    assert!(find("every.fetch").is_some());
    assert!(
        for_agent
            .iter()
            .all(|action| action.id != "extension.file.pick")
    );
    assert!(
        for_agent
            .iter()
            .any(|action| action.id == "extension.view.render")
    );

    Ok(())
}
