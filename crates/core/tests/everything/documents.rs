use std::fs;

use serde_json::{Value, json};

use super::Stage;
use super::stage::PROJECT;
use synt_testkit::TestError;
use synt_testkit::core::rejection;

async fn write(stage: &Stage, text: &str, base: Option<&str>) -> Result<Value, TestError> {
    stage
        .every(
            "write",
            json!({ "project": PROJECT, "path": "notes/plan.note", "text": text, "base": base }),
        )
        .await
}

async fn read(stage: &Stage) -> Result<Value, TestError> {
    stage
        .every(
            "read",
            json!({ "project": PROJECT, "path": "notes/plan.note" }),
        )
        .await
}

fn revision(written: &Value) -> Result<String, TestError> {
    let revision = written["info"]["revision"]
        .as_str()
        .ok_or("the document has no revision")?;

    Ok(revision.to_owned())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_note_is_written_into_the_project_and_read_back() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    let written = write(&stage, "one", None).await?;
    let found = read(&stage).await?;
    let on_disk = stage.project.join("extensions/every/notes/plan.note");

    assert_eq!(written["merged"], false);
    assert_eq!(written["info"]["kind"], "ext.every.note");
    assert_eq!(found["text"], "one");
    assert_eq!(found["info"]["revision"], written["info"]["revision"]);
    assert_eq!(fs::read_to_string(on_disk).unwrap(), "one");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn a_write_from_a_stale_revision_is_merged_by_the_extension() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let first = revision(&write(&stage, "one", None).await?)?;
    write(&stage, "one\ntwo", Some(&first)).await?;

    let late = write(&stage, "one\nthree", Some(&first)).await?;

    assert_eq!(late["merged"], true);
    assert_eq!(read(&stage).await?["text"], "one\ntwo\nthree");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn an_edit_made_outside_the_core_is_merged_too() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    let first = revision(&write(&stage, "one", None).await?)?;
    let on_disk = stage.project.join("extensions/every/notes/plan.note");
    fs::write(&on_disk, "one\nfrom git").unwrap();

    let written = write(&stage, "one\nfrom the app", Some(&first)).await?;

    assert_eq!(written["merged"], true);
    assert_eq!(
        fs::read_to_string(on_disk).unwrap(),
        "one\nfrom git\nfrom the app"
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn documents_are_listed_for_the_app_and_removed() -> Result<(), TestError> {
    let stage = Stage::start().await?;
    write(&stage, "one", None).await?;

    let listed = stage
        .call("extension.documents", json!({ "project": PROJECT }))
        .await?;
    let own = stage.every("list", json!({ "project": PROJECT })).await?;
    stage
        .every(
            "remove",
            json!({ "project": PROJECT, "path": "notes/plan.note" }),
        )
        .await?;

    assert_eq!(listed[0]["extension"], "every");
    assert_eq!(listed[0]["path"], "notes/plan.note");
    assert_eq!(own[0]["kind"], "ext.every.note");
    assert_eq!(
        stage.every("list", json!({ "project": PROJECT })).await?,
        json!([])
    );

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn only_its_own_kinds_inside_its_own_folder() -> Result<(), TestError> {
    let stage = Stage::start().await?;

    for (path, code) in [
        ("plan.txt", "not_allowed"),
        ("../escape.note", "invalid_path"),
        ("/etc/passwd.note", "invalid_path"),
    ] {
        let refused = stage
            .try_every(
                "write",
                json!({ "project": PROJECT, "path": path, "text": "x" }),
            )
            .await;

        assert!(rejection(refused).message.starts_with(code), "{path}");
    }

    let personal = stage
        .try_every(
            "write",
            json!({ "project": "personal", "path": "a.note", "text": "x" }),
        )
        .await;
    assert!(rejection(personal).message.starts_with("unknown_project"));

    Ok(())
}
