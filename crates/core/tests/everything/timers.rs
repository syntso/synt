use std::time::Duration;

use serde_json::{Value, json};
use synt_proto::ViewPatched;
use synt_testkit::TestError;
use tokio::time;

use super::Stage;
use super::stage::read;
use synt_testkit::core::eventually;

async fn beats(stage: &Stage) -> Result<u64, TestError> {
    let recalled = stage
        .every("recall", json!({ "key": "timer:beat" }))
        .await?;

    Ok(recalled["value"]
        .as_str()
        .and_then(|count| count.parse().ok())
        .unwrap_or(0))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_extension_starts_when_it_is_loaded() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    eventually("the start", async || {
        let heard = stage.every("heard", json!({})).await.ok()?;

        (heard["started"] == true).then_some(())
    })
    .await;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_timer_fires_until_it_is_cleared_and_patches_the_status_item() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let (_watcher, mut notifications) = stage.watcher().await?;

    stage
        .every("arm", json!({ "name": "beat", "seconds": 1 }))
        .await?;
    eventually("two beats", async || {
        (beats(&stage).await.ok()? >= 2).then_some(())
    })
    .await;

    let patched = loop {
        let notification = notifications.recv().await.unwrap();
        if let Some(patched) = read::<ViewPatched>(&notification) {
            break patched;
        }
    };
    assert_eq!(patched.extension, "every");
    assert_eq!(patched.view, "beat");
    assert_eq!(patched.patch[0]["path"], "/text");

    stage.every("disarm", json!({ "name": "beat" })).await?;
    let stopped_at = beats(&stage).await?;
    time::sleep(Duration::from_millis(2500)).await;

    assert!(beats(&stage).await? <= stopped_at + 1);

    let status = stage
        .call(
            "extension.view.render",
            json!({ "extension": "every", "view": "beat" }),
        )
        .await?;
    assert!(
        status["tree"]["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("beats: ")),
        "{status}"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn timers_outside_the_limits_are_refused() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let refused = stage
        .try_every("arm", json!({ "name": "beat", "seconds": 0 }))
        .await;

    let error: Value = json!(synt_testkit::core::rejection(refused).message);
    assert!(
        error.as_str().unwrap().starts_with("invalid_timer"),
        "{error}"
    );

    Ok(())
}
