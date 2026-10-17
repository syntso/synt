use std::fs;

use serde_json::json;
use synt_proto::ErrorCode;
use synt_testkit::TestError;

use super::Stage;
use synt_testkit::core::rejection;

#[tokio::test(flavor = "multi_thread")]
async fn a_picked_file_is_read_and_written_by_handle() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let path = stage.harness.grounds().path().join("diagram.txt");
    fs::write(&path, "first draft").unwrap();

    let picked = stage
        .call(
            "extension.file.pick",
            json!({ "extension": "every", "path": path, "access": "read-write" }),
        )
        .await?;
    let handle = picked["handle"].as_str().unwrap();

    let chosen = stage.every("files", json!({})).await?;
    let read = stage
        .every("read_file", json!({ "handle": handle }))
        .await?;
    stage
        .every(
            "write_file",
            json!({ "handle": handle, "text": "second draft" }),
        )
        .await?;

    assert_eq!(picked["name"], "diagram.txt");
    assert_eq!(
        chosen,
        json!([{ "handle": handle, "name": "diagram.txt", "writable": true }])
    );
    assert_eq!(read["text"], "first draft");
    assert_eq!(fs::read_to_string(&path).unwrap(), "second draft");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_file_picked_for_reading_cannot_be_written_or_used_after_it_is_taken_back()
-> Result<(), TestError> {
    let stage = Stage::start().await?;
    let path = stage.harness.grounds().path().join("readme.txt");
    fs::write(&path, "read me").unwrap();

    let picked = stage
        .call(
            "extension.file.pick",
            json!({ "extension": "every", "path": path, "access": "read" }),
        )
        .await?;
    let handle = picked["handle"].as_str().unwrap();

    let write = stage
        .try_every("write_file", json!({ "handle": handle, "text": "changed" }))
        .await;
    stage
        .call(
            "extension.file.forget",
            json!({ "extension": "every", "handle": handle }),
        )
        .await?;
    let read = stage
        .try_every("read_file", json!({ "handle": handle }))
        .await;

    assert!(rejection(write).message.starts_with("not_allowed"));
    assert!(rejection(read).message.starts_with("unknown_file"));
    assert_eq!(fs::read_to_string(&path).unwrap(), "read me");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn only_the_person_picks_files() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let agent = stage.harness.agent("helper").await?;
    let call = synt_proto::ActionCall::new("extension.file.pick").params(json!({
        "extension": "every",
        "path": "/etc/hosts",
        "access": "read",
    }));

    let error = rejection(agent.call(call).await);

    assert_eq!(error.code, ErrorCode::DENIED);

    Ok(())
}
