#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::{PROJECT_SLUG, Team};
use synt_sync::{DocumentKind, DocumentMerger, DocumentPath};
use tokio::time;

const DIAGRAM: &str = "extensions/uml/orders.uml";

struct Concatenate;

impl DocumentMerger for Concatenate {
    fn merge(
        &self,
        kind: &DocumentKind,
        _base: Option<&[u8]>,
        older: &[u8],
        newer: &[u8],
    ) -> Option<Vec<u8>> {
        assert_eq!(kind.to_string(), "ext.uml.uml");

        Some([older, b"\n", newer].concat())
    }
}

async fn edit_on_both(team: &Team) {
    team.alice.write(DIAGRAM, "@startuml\n@enduml\n");
    team.settle().await;

    team.alice.write(DIAGRAM, "left");
    time::sleep(Duration::from_millis(5)).await;
    team.bob.write(DIAGRAM, "right");
    team.settle().await;
    team.bob.sync().await;
}

#[tokio::test]
async fn extension_documents_sync_like_any_other() {
    let team = Team::new().await;

    team.alice.write(DIAGRAM, "@startuml\nA -> B\n@enduml\n");
    team.settle().await;

    assert_eq!(
        team.bob.read(DIAGRAM).as_deref(),
        Some("@startuml\nA -> B\n@enduml\n")
    );
}

#[tokio::test]
async fn without_a_merger_both_versions_are_kept_until_the_person_picks_one() {
    let team = Team::new().await;

    edit_on_both(&team).await;

    assert_eq!(team.alice.read(DIAGRAM).as_deref(), Some("left"));
    assert_eq!(team.bob.read(DIAGRAM).as_deref(), Some("right"));

    let conflict = team.alice.status().await.conflicts.remove(0);
    assert_eq!(conflict.path, DIAGRAM);
    assert_eq!(conflict.merged, None);
    assert_eq!(conflict.candidates.len(), 2);

    let bob = team.bob.member().await;
    let history = team.alice.history(DIAGRAM).await.unwrap();
    let theirs = history
        .iter()
        .find(|revision| revision.head && revision.author.as_ref() == Some(&bob))
        .unwrap();

    team.alice
        .syncer()
        .resolve(
            PROJECT_SLUG,
            DocumentPath::parse(DIAGRAM).unwrap(),
            theirs.revision.clone(),
        )
        .await
        .unwrap();
    team.settle().await;

    for device in [&team.alice, &team.bob] {
        assert_eq!(device.read(DIAGRAM).as_deref(), Some("right"));
        assert!(device.status().await.conflicts.is_empty());
    }
}

#[tokio::test]
async fn the_owning_extension_merges_its_documents() {
    let team = Team::with_merger(Arc::new(Concatenate)).await;

    edit_on_both(&team).await;

    for device in [&team.alice, &team.bob] {
        assert_eq!(device.read(DIAGRAM).as_deref(), Some("left\nright"));
        assert!(device.status().await.conflicts.is_empty());
    }
}
