use std::fs;

use synt_testkit::TestError;
use synt_testkit::sync::Team;

const API: &str = "kind = \"host\"\naddress = \"10.0.1.12\"\nuser = \"deploy\"\n";
const DB: &str = "kind = \"host\"\naddress = \"10.0.1.20\"\nuser = \"deploy\"\n";

async fn team_with_two_resources() -> Result<Team, TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/api-1.toml", API)?;
    team.alice.write("resources/db.toml", DB)?;
    team.settle().await;
    team.bob.sync().await;

    Ok(team)
}

async fn bob_still_has_both(team: &Team) {
    team.bob.sync().await;

    assert_eq!(team.bob.read("resources/api-1.toml").as_deref(), Some(API));
    assert_eq!(team.bob.read("resources/db.toml").as_deref(), Some(DB));
}

#[tokio::test]
async fn a_moved_project_folder_removes_nothing_for_the_others() -> Result<(), TestError> {
    let team = team_with_two_resources().await?;
    let folder = team.alice.project();
    let moved = folder.with_file_name("moved");

    fs::rename(&folder, &moved).unwrap();
    let report = team.alice.round().await;

    assert!(report.error.unwrap().contains("not a plain folder"));
    assert_eq!(report.pushed, 0);
    bob_still_has_both(&team).await;

    fs::rename(&moved, &folder).unwrap();
    let back = team.alice.sync().await;

    assert_eq!(back.recorded, 0);

    Ok(())
}

#[tokio::test]
async fn a_folder_that_holds_another_project_is_not_synced() -> Result<(), TestError> {
    let team = team_with_two_resources().await?;
    let folder = team.alice.project();

    fs::remove_dir_all(&folder).unwrap();
    team.alice.write(
        "synt.toml",
        "id = \"0123456789abcdef0123456789abcdef\"\nname = \"Other\"\n",
    )?;
    let report = team.alice.round().await;

    assert!(report.error.unwrap().contains("project file"));
    assert_eq!(report.pushed, 0);
    bob_still_has_both(&team).await;

    Ok(())
}

#[tokio::test]
async fn a_project_file_that_is_gone_stops_the_round() -> Result<(), TestError> {
    let team = team_with_two_resources().await?;

    team.alice.remove("synt.toml")?;
    let report = team.alice.round().await;

    assert!(report.error.unwrap().contains("project file"));
    bob_still_has_both(&team).await;

    Ok(())
}

#[tokio::test]
async fn removing_every_document_at_once_is_refused() -> Result<(), TestError> {
    let team = team_with_two_resources().await?;

    team.alice.remove("resources/api-1.toml")?;
    team.alice.remove("resources/db.toml")?;
    let report = team.alice.round().await;

    assert!(
        report
            .error
            .unwrap()
            .contains("every one of the 2 documents")
    );
    assert_eq!(report.pushed, 0);
    bob_still_has_both(&team).await;

    Ok(())
}

#[tokio::test]
async fn documents_removed_a_few_at_a_time_are_removed_for_everyone() -> Result<(), TestError> {
    let team = team_with_two_resources().await?;

    team.alice.remove("resources/api-1.toml")?;
    team.alice.sync().await;
    team.alice.remove("resources/db.toml")?;
    team.alice.sync().await;
    team.bob.sync().await;

    assert_eq!(team.bob.read("resources/api-1.toml"), None);
    assert_eq!(team.bob.read("resources/db.toml"), None);

    Ok(())
}
