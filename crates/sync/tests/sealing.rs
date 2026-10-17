use std::fs;
use std::path::{Path, PathBuf};

use synt_testkit::TestError;
use synt_testkit::sync::{PROJECT_ID, Team};

const SECRET_HOST: &str = "kind = \"host\"\naddress = \"db-primary.internal.example\"\n\
                           user = \"deploy\"\nvia = \"bastion\"\n";

fn files(root: &Path) -> Result<Vec<PathBuf>, TestError> {
    let mut found = Vec::new();

    for entry in fs::read_dir(root)? {
        let path = entry?.path();

        if path.is_dir() {
            found.extend(files(&path)?);
        } else {
            found.push(path);
        }
    }

    Ok(found)
}

#[tokio::test]
async fn the_shared_folder_holds_only_ciphertext() -> Result<(), TestError> {
    let team = Team::new().await?;
    team.alice.write("resources/primary-db.toml", SECRET_HOST)?;
    team.alice.write(
        "runbooks/failover.toml",
        "name = \"Fail over the orders database\"\n",
    )?;
    team.settle().await;

    let words = [
        "db-primary",
        "internal.example",
        "primary-db",
        "bastion",
        "deploy",
        "resources",
        "runbooks",
        "failover",
        "Orders",
        "synt.toml",
    ];
    let stored = files(team.shared())?;

    assert!(stored.len() > 4);
    for file in stored {
        let text = String::from_utf8_lossy(&fs::read(&file).unwrap()).to_lowercase();
        let name = file.to_string_lossy().to_lowercase();

        for word in words {
            let word = word.to_lowercase();

            assert!(!text.contains(&word), "{} shows {word}", file.display());
            assert!(
                !name.contains(&word),
                "{} is named after {word}",
                file.display()
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn a_tampered_revision_is_refused() -> Result<(), TestError> {
    let team = Team::new().await?;
    let carol = team.device()?;
    carol.enable().await?;
    team.alice.share_with(&carol).await?;

    team.alice.write("resources/primary-db.toml", SECRET_HOST)?;
    team.alice.sync().await;

    let revisions = team.shared().join(PROJECT_ID).join("revisions");
    for file in files(&revisions)? {
        let mut sealed: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        let text = sealed["sealed"].as_str().unwrap().to_owned();
        let flipped = if text.starts_with('A') { "B" } else { "A" };
        sealed["sealed"] = format!("{flipped}{}", &text[1..]).into();

        fs::write(&file, serde_json::to_vec(&sealed).unwrap()).unwrap();
    }

    let report = carol.sync().await;

    assert!(report.refused > 0);
    assert!(!report.problems.is_empty());
    assert_eq!(carol.read("resources/primary-db.toml"), None);

    Ok(())
}
