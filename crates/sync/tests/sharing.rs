use std::fs;

use synt_sync::{Access, Generation, Place};
use synt_testkit::TestError;
use synt_testkit::sync::{Device, PROJECT_ID, Team, project_slug};

const API: &str = "kind = \"host\"\naddress = \"10.0.1.12\"\nuser = \"deploy\"\n";

#[tokio::test]
async fn the_first_device_holds_the_key_and_is_the_only_member() -> Result<(), TestError> {
    let shared = tempfile::tempdir().unwrap();
    let alice = Device::new(shared.path())?;

    let status = alice.enable().await?;

    assert_eq!(status.access, Access::Member);
    assert_eq!(status.generation, Some(Generation::FIRST));
    assert_eq!(status.members.len(), 1);
    assert!(status.members[0].this_device);
    assert!(
        alice
            .vault_items()
            .contains(&format!("sync.project.{PROJECT_ID}"))
    );
    assert!(alice.vault_items().contains(&"sync.device".to_owned()));

    Ok(())
}

#[tokio::test]
async fn a_new_device_waits_until_a_member_shares_the_key() -> Result<(), TestError> {
    let shared = tempfile::tempdir().unwrap();
    let alice = Device::new(shared.path())?;
    let bob = Device::new(shared.path())?;
    alice.enable().await?;
    alice.write("resources/api-1.toml", API)?;
    alice.sync().await;

    let waiting = bob.enable().await?;
    bob.sync().await;

    assert_eq!(waiting.access, Access::Waiting);
    assert_eq!(waiting.generation, None);
    assert_eq!(bob.read("resources/api-1.toml"), None);

    let pending = alice.status().await?.members;
    assert!(
        pending
            .iter()
            .any(|member| member.member == waiting.device && member.access == Access::Waiting)
    );

    alice.share_with(&bob).await?;
    bob.sync().await;

    assert_eq!(bob.status().await?.access, Access::Member);
    assert_eq!(bob.read("resources/api-1.toml").as_deref(), Some(API));

    Ok(())
}

#[tokio::test]
async fn what_a_waiting_device_already_had_is_merged_in_once_it_joins() -> Result<(), TestError> {
    let shared = tempfile::tempdir().unwrap();
    let alice = Device::new(shared.path())?;
    let bob = Device::new(shared.path())?;
    alice.enable().await?;
    alice.write("resources/api-1.toml", API)?;
    alice.sync().await;

    bob.write(
        "resources/db.toml",
        "kind = \"host\"\naddress = \"10.0.2.2\"\n",
    )?;
    bob.enable().await?;
    alice.share_with(&bob).await?;
    bob.sync().await;
    alice.sync().await;

    assert!(alice.read("resources/db.toml").is_some());
    assert!(bob.read("resources/api-1.toml").is_some());

    Ok(())
}

#[tokio::test]
async fn sharing_needs_the_member_to_have_asked_to_join() -> Result<(), TestError> {
    let shared = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let alice = Device::new(shared.path())?;
    let stranger = Device::new(elsewhere.path())?;
    alice.enable().await?;
    stranger.enable().await?;

    let member = stranger.member().await?;
    let error = alice
        .syncer()
        .share(&project_slug()?, member)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("has not joined"), "{error}");

    Ok(())
}

#[tokio::test]
async fn removing_a_member_rotates_the_key_and_locks_them_out_of_new_revisions()
-> Result<(), TestError> {
    let team = Team::new().await?;
    let carol = team.device()?;
    carol.enable().await?;
    team.alice.share_with(&carol).await?;
    carol.sync().await;
    assert_eq!(carol.status().await?.access, Access::Member);

    let removed = carol.member().await?;
    let change = team
        .alice
        .syncer()
        .unshare(&project_slug()?, removed.clone())
        .await
        .unwrap();

    assert_eq!(change.generation, Generation::new(2));

    team.alice.write("resources/api-1.toml", API)?;
    team.alice.sync().await;
    team.bob.sync().await;
    carol.sync().await;

    assert_eq!(team.bob.read("resources/api-1.toml").as_deref(), Some(API));
    assert_eq!(
        team.bob.status().await?.generation,
        Some(Generation::new(2))
    );
    assert_eq!(carol.read("resources/api-1.toml"), None);
    assert_eq!(carol.status().await?.access, Access::Removed);
    assert!(
        team.alice
            .status()
            .await?
            .members
            .iter()
            .all(|member| member.member != removed)
    );

    Ok(())
}

#[tokio::test]
async fn revisions_after_a_rotation_are_sealed_with_the_new_generation() -> Result<(), TestError> {
    let team = Team::new().await?;
    let carol = team.device()?;
    carol.enable().await?;
    let removed = carol.member().await?;
    team.alice
        .syncer()
        .unshare(&project_slug()?, removed)
        .await
        .unwrap();

    team.alice.write("resources/api-1.toml", API)?;
    team.alice.sync().await;

    let revisions = team.shared().join(PROJECT_ID).join("revisions");
    let newest = fs::read_dir(&revisions)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .max()
        .unwrap();
    let sealed: serde_json::Value = serde_json::from_slice(&fs::read(newest).unwrap()).unwrap();

    assert_eq!(sealed["generation"], 2);

    Ok(())
}

#[tokio::test]
async fn a_device_cannot_remove_itself() -> Result<(), TestError> {
    let team = Team::new().await?;
    let me = team.alice.member().await?;

    let error = team
        .alice
        .syncer()
        .unshare(&project_slug()?, me)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("sync.disable"), "{error}");

    Ok(())
}

#[tokio::test]
async fn disabling_forgets_the_key_and_leaves_the_project() -> Result<(), TestError> {
    let team = Team::new().await?;
    let bob = team.bob.member().await?;

    let disabled = team.bob.syncer().disable(&project_slug()?).await.unwrap();

    assert!(disabled.left);
    assert_eq!(disabled.device, bob);
    assert!(
        !team
            .bob
            .vault_items()
            .iter()
            .any(|item| item.starts_with("sync.project."))
    );
    assert!(team.bob.syncer().status(&project_slug()?).await.is_err());
    assert!(
        team.alice
            .status()
            .await?
            .members
            .iter()
            .all(|member| member.member != bob)
    );

    Ok(())
}

#[tokio::test]
async fn a_project_syncs_through_one_folder_at_a_time() -> Result<(), TestError> {
    let team = Team::new().await?;
    let elsewhere = tempfile::tempdir().unwrap();

    let error = team
        .alice
        .syncer()
        .enable(&project_slug()?, Place::Folder(elsewhere.path().to_owned()))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("already syncs"), "{error}");

    Ok(())
}

#[tokio::test]
async fn a_missing_sync_folder_is_refused() -> Result<(), TestError> {
    let shared = tempfile::tempdir().unwrap();
    let alice = Device::new(shared.path())?;
    let missing = shared.path().join("missing");

    let error = alice
        .syncer()
        .enable(&project_slug()?, Place::Folder(missing))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("does not exist"), "{error}");

    Ok(())
}
