use std::time::Duration;

use synt_sync::Access;
use synt_testkit::TestError;
use synt_testkit::sync::{Device, Team, project_slug};
use tokio::time;

const API: &str = "kind = \"host\"\naddress = \"10.0.1.12\"\nuser = \"deploy\"\nport = 22\n";

fn table(text: &str) -> Result<toml::Table, toml::de::Error> {
    text.parse()
}

async fn later() {
    time::sleep(Duration::from_millis(5)).await;
}

#[tokio::test]
async fn an_edit_on_one_device_reaches_the_other_and_back() -> Result<(), TestError> {
    let team = Team::new().await?;

    team.alice.write("resources/api-1.toml", API)?;
    team.alice.sync().await;
    team.bob.sync().await;

    assert_eq!(team.bob.read("resources/api-1.toml").as_deref(), Some(API));

    let moved = API.replace("10.0.1.12", "10.0.9.9");
    team.bob.write("resources/api-1.toml", &moved)?;
    team.bob.sync().await;
    team.alice.sync().await;

    assert_eq!(team.alice.read("resources/api-1.toml"), Some(moved));

    Ok(())
}

#[tokio::test]
async fn runbooks_and_the_project_file_sync_too() -> Result<(), TestError> {
    let team = Team::new().await?;
    let runbook = "name = \"Restart API\"\n\n[[steps]]\naction = \"ssh.exec\"\n";

    team.alice.write("runbooks/restart.toml", runbook)?;
    team.alice.write(
        "synt.toml",
        "id = \"4f2c9a7e1b3d5f60718293a4b5c6d7e8\"\nname = \"Shop\"\n",
    )?;
    team.settle().await;

    assert_eq!(
        team.bob.read("runbooks/restart.toml").as_deref(),
        Some(runbook)
    );
    assert!(team.bob.read("synt.toml").unwrap().contains("Shop"));

    Ok(())
}

#[tokio::test]
async fn a_removed_document_is_removed_on_the_other_device() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    team.bob.remove("resources/api-1.toml")?;
    team.bob.sync().await;
    team.alice.sync().await;

    assert_eq!(team.alice.read("resources/api-1.toml"), None);

    Ok(())
}

#[tokio::test]
async fn edits_of_different_fields_at_the_same_time_merge() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    team.alice.write(
        "resources/api-1.toml",
        &API.replace("10.0.1.12", "10.0.9.9"),
    )?;
    team.bob
        .write("resources/api-1.toml", &API.replace("deploy", "ops"))?;
    team.alice.sync().await;
    team.bob.sync().await;
    team.alice.sync().await;

    let expected = table(
        &API.replace("10.0.1.12", "10.0.9.9")
            .replace("deploy", "ops"),
    )?;
    let on_alice = team.alice.read("resources/api-1.toml").unwrap();
    let on_bob = team.bob.read("resources/api-1.toml").unwrap();

    assert_eq!(table(&on_alice)?, expected);
    assert_eq!(on_alice, on_bob);
    assert!(team.alice.status().await?.conflicts.is_empty());
    assert!(team.bob.status().await?.conflicts.is_empty());

    Ok(())
}

#[tokio::test]
async fn the_merge_is_the_same_revision_on_both_devices() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    team.alice.write(
        "resources/api-1.toml",
        &API.replace("10.0.1.12", "10.0.9.9"),
    )?;
    team.bob
        .write("resources/api-1.toml", &API.replace("deploy", "ops"))?;
    team.alice.sync().await;
    team.bob.sync().await;
    team.alice.sync().await;
    team.bob.sync().await;

    let on_alice = team.alice.history("resources/api-1.toml").await.unwrap();
    let on_bob = team.bob.history("resources/api-1.toml").await.unwrap();
    let heads = |history: &[synt_sync::RevisionInfo]| -> Vec<String> {
        history
            .iter()
            .filter(|revision| revision.head)
            .map(|revision| revision.revision.to_string())
            .collect()
    };

    assert_eq!(heads(&on_alice).len(), 1);
    assert_eq!(heads(&on_alice), heads(&on_bob));
    assert_eq!(on_alice.len(), on_bob.len());

    Ok(())
}

#[tokio::test]
async fn the_same_field_changed_twice_keeps_the_newer_value_and_reports_a_conflict()
-> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    team.alice.write(
        "resources/api-1.toml",
        &API.replace("port = 22", "port = 2222"),
    )?;
    later().await;
    team.bob.write(
        "resources/api-1.toml",
        &API.replace("port = 22", "port = 2200"),
    )?;
    team.settle().await;
    team.bob.sync().await;

    for device in [&team.alice, &team.bob] {
        let file = table(&device.read("resources/api-1.toml").unwrap())?;
        let conflicts = device.status().await?.conflicts;

        assert_eq!(file["port"].as_integer(), Some(2200));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].path.get(), "resources/api-1.toml");
        assert_eq!(conflicts[0].fields, vec!["port"]);
        assert_eq!(conflicts[0].candidates.len(), 2);
        assert!(conflicts[0].merged.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn a_conflict_is_resolved_by_keeping_one_revision() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    team.alice.write(
        "resources/api-1.toml",
        &API.replace("port = 22", "port = 2222"),
    )?;
    later().await;
    team.bob.write(
        "resources/api-1.toml",
        &API.replace("port = 22", "port = 2200"),
    )?;
    team.settle().await;

    let alice = team.alice.member().await?;
    let history = team.alice.history("resources/api-1.toml").await.unwrap();
    let conflict = team.alice.status().await?.conflicts.remove(0);
    let mine = history
        .iter()
        .find(|revision| {
            conflict.candidates.contains(&revision.revision)
                && revision.author.as_ref() == Some(&alice)
        })
        .unwrap();

    let resolution = team
        .alice
        .syncer()
        .resolve(
            &project_slug()?,
            synt_sync::DocumentPath::parse("resources/api-1.toml").unwrap(),
            mine.revision.clone(),
        )
        .await
        .unwrap();
    team.settle().await;

    assert!(resolution.head);
    assert!(resolution.conflicts.is_empty());
    for device in [&team.alice, &team.bob] {
        let file = table(&device.read("resources/api-1.toml").unwrap())?;

        assert_eq!(file["port"].as_integer(), Some(2222));
        assert!(device.status().await?.conflicts.is_empty());
    }

    Ok(())
}

#[tokio::test]
async fn a_document_without_a_conflict_cannot_be_resolved() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    let history = team.alice.history("resources/api-1.toml").await.unwrap();
    let error = team
        .alice
        .syncer()
        .resolve(
            &project_slug()?,
            synt_sync::DocumentPath::parse("resources/api-1.toml").unwrap(),
            history[0].revision.clone(),
        )
        .await
        .unwrap_err();

    assert!(error.to_string().contains("no conflict"), "{error}");

    Ok(())
}

#[tokio::test]
async fn an_edit_against_a_removal_keeps_the_edit() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    team.alice.remove("resources/api-1.toml")?;
    team.bob
        .write("resources/api-1.toml", &API.replace("deploy", "ops"))?;
    team.settle().await;
    team.bob.sync().await;

    for device in [&team.alice, &team.bob] {
        assert!(device.read("resources/api-1.toml").unwrap().contains("ops"));
        assert_eq!(device.status().await?.conflicts.len(), 1);
    }

    Ok(())
}

#[tokio::test]
async fn the_history_lists_revisions_newest_first() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.alice.sync().await;
    later().await;
    team.alice
        .write("resources/api-1.toml", &API.replace("deploy", "ops"))?;
    team.alice.sync().await;

    let history = team.alice.history("resources/api-1.toml").await.unwrap();

    assert_eq!(history.len(), 2);
    assert!(history[0].head && history[0].on_disk);
    assert_eq!(history[0].parents, vec![history[1].revision.clone()]);
    assert!(history[0].at >= history[1].at);
    assert_eq!(history[1].bytes, API.len());

    Ok(())
}

#[tokio::test]
async fn files_that_are_not_documents_stay_local() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("notes.md", "private notes")?;
    team.alice.write("resources/readme.txt", "text")?;
    team.settle().await;

    assert_eq!(team.bob.read("notes.md"), None);
    assert_eq!(team.bob.read("resources/readme.txt"), None);
    assert_eq!(team.alice.status().await?.access, Access::Member);

    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn a_document_that_cannot_be_read_does_not_hold_up_the_others() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.settle().await;

    let linked = team.bob.project().join("resources/api-1.toml");
    std::fs::remove_file(&linked).unwrap();
    std::os::unix::fs::symlink("/etc/hosts", &linked).unwrap();

    team.alice
        .write("resources/api-1.toml", &API.replace("deploy", "ops"))?;
    team.alice.write(
        "resources/db.toml",
        "kind = \"host\"\naddress = \"10.0.2.2\"\n",
    )?;
    team.alice.sync().await;
    let report = team.bob.sync().await;

    assert!(
        report
            .problems
            .iter()
            .any(|problem| problem.contains("api-1")),
        "{:?}",
        report.problems
    );
    assert!(team.bob.read("resources/db.toml").is_some());
    assert!(linked.symlink_metadata().unwrap().file_type().is_symlink());

    Ok(())
}

#[tokio::test]
async fn a_merge_that_changes_nothing_on_disk_is_not_reported_as_applied() -> Result<(), TestError>
{
    let shared = tempfile::tempdir().unwrap();
    let alice = Device::new(shared.path())?;
    let bob = Device::new(shared.path())?;
    alice.enable().await?;
    bob.enable().await?;
    alice.share_with(&bob).await?;

    alice.sync().await;
    let joined = bob.sync().await;
    let merged = alice.sync().await;

    assert_eq!(joined.merged, 1);
    assert!(joined.applied.is_empty(), "{:?}", joined.applied);
    assert!(merged.applied.is_empty(), "{:?}", merged.applied);

    Ok(())
}
